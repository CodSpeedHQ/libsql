use std::sync::Arc;

use roaring::RoaringBitmap;
use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt};

use crate::error::Result;
use crate::io::Io;
use crate::segment::Frame;
use crate::shared_wal::SharedWal;

pub struct Replicator<IO: Io> {
    shared: Arc<SharedWal<IO>>,
    new_frame_notifier: watch::Receiver<u64>,
    next_frame_no: u64,
}

impl<IO: Io> Replicator<IO> {
    pub fn new(shared: Arc<SharedWal<IO>>, next_frame_no: u64) -> Self {
        let new_frame_notifier = shared.new_frame_notifier.subscribe();
        Self {
            shared,
            new_frame_notifier,
            next_frame_no,
        }
    }

    /// Stream frames from this replicator. The replicator will wait for new frames to become
    /// available, and never return.
    ///
    /// The replicator keeps track of how much progress has been made by the replica, and will
    /// attempt to find the next frames to send with following strategy:
    /// - First, replicate as much as possible from the current log
    /// - The, if we still haven't caught up with `self.start_frame_no`, we select the next frames
    /// to replicate from tail of current.
    /// - Finally, if we still haven't reached `self.start_frame_no`, read from durable storage
    /// (todo: maybe the replica should read from durable storage directly?)
    ///
    /// In a single replication step, the replicator guarantees that a minimal set of frames is
    /// sent to the replica.
    pub fn frame_stream(&mut self) -> impl Stream<Item = Result<Frame>> + '_ {
        async_stream::try_stream! {
            loop {
                let most_recent_frame_no = *self
                    .new_frame_notifier
                    .wait_for(|fno| *fno > self.next_frame_no)
                    .await
                    .expect("channel cannot be closed because we hold a ref to the sending end");

                let mut commit_frame_no = 0;
                if most_recent_frame_no > self.next_frame_no {
                    let current = self.shared.current.load();
                    let mut seen = RoaringBitmap::new();
                    let (stream, replicated_until, size_after) = current.frame_stream_from(self.next_frame_no, &mut seen);
                    let should_replicate_from_tail = replicated_until != self.next_frame_no;

                    // replicate from current
                    {
                        tokio::pin!(stream);

                        let mut stream = stream.peekable();

                        loop {
                            let Some(frame) = stream.next().await else { break };
                            let mut frame = frame?;
                            commit_frame_no = frame.header().frame_no().max(commit_frame_no);
                            if stream.peek().await.is_none() && !should_replicate_from_tail {
                                frame.header_mut().set_size_after(size_after);
                                self.next_frame_no = commit_frame_no + 1;
                            }

                            yield frame
                        }
                    }


                    // replicate from tail
                    if should_replicate_from_tail {
                        let (stream, replicated_until) = current.tail().stream_pages_from(self.next_frame_no, &mut seen).await;
                        tokio::pin!(stream);
                        let mut stream = stream.peekable();

                        let should_replicate_from_durable = replicated_until != self.next_frame_no;

                        loop {
                            let Some(frame) = stream.next().await else { break };
                            let mut frame = frame?;
                            commit_frame_no = frame.header().frame_no().max(commit_frame_no);
                            if stream.peek().await.is_none() && !should_replicate_from_durable {
                                frame.header_mut().set_size_after(size_after);
                                self.next_frame_no = commit_frame_no + 1;
                            }

                            yield frame
                        }

                        if should_replicate_from_durable {
                            todo!("we need to fetch new segments from durable storage");
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use tempfile::NamedTempFile;
    use tokio::time::timeout;
    use tokio_stream::StreamExt;

    use crate::io::FileExt;
    use crate::test::{seal_current_segment, TestEnv};

    use super::*;

    #[tokio::test]
    async fn stream_from_current_log() {
        let env = TestEnv::new();
        let conn = env.open_conn("test");
        let shared = env.shared("test");

        conn.execute("create table test (x)", ()).unwrap();

        for _ in 0..50 {
            conn.execute("insert into test values (randomblob(128))", ())
                .unwrap();
        }

        let mut replicator = Replicator::new(shared.clone(), 1);

        let tmp = NamedTempFile::new().unwrap();
        let stream = replicator.frame_stream();
        tokio::pin!(stream);
        let mut last_frame_no = 0;
        let mut size_after = 0;
        loop {
            let fut = timeout(Duration::from_millis(15), stream.next());
            let Ok(Some(frame)) = fut.await else { break };
            let frame = frame.unwrap();
            // the last frame should commit
            size_after = frame.header().size_after();
            last_frame_no = last_frame_no.max(frame.header().frame_no());
            let offset = (frame.header().page_no() - 1) * 4096;
            tmp.as_file()
                .write_all_at(frame.data(), offset as _)
                .unwrap();
        }

        assert_eq!(size_after, 4);
        assert_eq!(last_frame_no, 55);

        {
            let conn = libsql_sys::rusqlite::Connection::open(tmp.path()).unwrap();
            conn.query_row("select count(0) from test", (), |row| {
                let count = row.get_unwrap::<_, usize>(0);
                assert_eq!(count, 50);
                Ok(())
            })
            .unwrap();
        }

        seal_current_segment(&shared);

        for _ in 0..50 {
            conn.execute("insert into test values (randomblob(128))", ())
                .unwrap();
        }

        let mut size_after = 0;
        loop {
            let fut = timeout(Duration::from_millis(15), stream.next());
            let Ok(Some(frame)) = fut.await else { break };
            let frame = frame.unwrap();
            assert!(frame.header().frame_no() > last_frame_no);
            size_after = frame.header().size_after();
            // the last frame should commit
            let offset = (frame.header().page_no() - 1) * 4096;
            tmp.as_file()
                .write_all_at(frame.data(), offset as _)
                .unwrap();
        }

        assert_eq!(size_after, 6);

        {
            let conn = libsql_sys::rusqlite::Connection::open(tmp.path()).unwrap();
            conn.query_row("select count(0) from test", (), |row| {
                let count = row.get_unwrap::<_, usize>(0);
                assert_eq!(count, 100);
                Ok(())
            })
            .unwrap();
        }

        // replicate everything from scratch again
        {
            let tmp = NamedTempFile::new().unwrap();
            let mut replicator = Replicator::new(shared.clone(), 1);
            let stream = replicator.frame_stream();

            tokio::pin!(stream);

            loop {
                let fut = timeout(Duration::from_millis(15), stream.next());
                let Ok(Some(frame)) = fut.await else { break };
                let frame = frame.unwrap();
                // the last frame should commit
                let offset = (frame.header().page_no() - 1) * 4096;
                tmp.as_file()
                    .write_all_at(frame.data(), offset as _)
                    .unwrap();
            }

            let conn = libsql_sys::rusqlite::Connection::open(tmp.path()).unwrap();
            conn.query_row("select count(0) from test", (), |row| {
                let count = row.get_unwrap::<_, usize>(0);
                assert_eq!(count, 100);
                Ok(())
            })
            .unwrap();
        }
    }
}
