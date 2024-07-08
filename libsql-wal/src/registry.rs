use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use hashbrown::hash_map::Entry;
use hashbrown::HashMap;
use libsql_sys::ffi::Sqlite3DbHeader;
use parking_lot::{Condvar, Mutex, RwLock, RwLockUpgradableReadGuard};
use zerocopy::{AsBytes, FromZeroes};

use crate::error::Result;
use crate::io::file::FileExt;
use crate::io::{Io, StdIO};
use crate::segment::list::SegmentList;
use crate::segment::{current::CurrentSegment, sealed::SealedSegment};
use crate::shared_wal::SharedWal;
use crate::transaction::{Transaction, TxGuard};
use libsql_sys::name::{NamespaceName, NamespaceResolver};

/// called on every segment on swap.
pub trait SegmentSwapHandler<F>: Send + Sync + 'static {
    fn handle_segment_swap(&self, namespace: NamespaceName, log: Arc<SealedSegment<F>>);
}

// ignore segments
impl<F> SegmentSwapHandler<F> for () {
    fn handle_segment_swap(&self, _namespace: NamespaceName, _log: Arc<SealedSegment<F>>) {}
}

enum Slot<FS: Io> {
    Wal(Arc<SharedWal<FS>>),
    /// Only a single thread is allowed to instantiate the wal. The first thread to acquire an
    /// entry in the registry map puts a building slot. Other connections will wait for the mutex
    /// to turn to true, after the slot has been updated to contain the wal
    Building(Arc<(Condvar, Mutex<bool>)>),
}

/// Wal Registry maintains a set of shared Wal, and their respective set of files.
pub struct WalRegistry<IO: Io> {
    fs: IO,
    path: PathBuf,
    shutdown: AtomicBool,
    opened: RwLock<HashMap<NamespaceName, Slot<IO>>>,
    resolver: Box<dyn NamespaceResolver + Send + Sync + 'static>,
    swap_handler: Box<dyn SegmentSwapHandler<IO::File>>,
}

impl WalRegistry<StdIO> {
    pub fn new(
        path: PathBuf,
        resolver: impl NamespaceResolver,
        swap_handler: impl SegmentSwapHandler<<StdIO as Io>::File>,
    ) -> Result<Self> {
        Self::new_with_fs(StdIO(()), path, resolver, swap_handler)
    }
}

impl<FS: Io> WalRegistry<FS> {
    pub fn new_with_fs(
        fs: FS,
        path: PathBuf,
        resolver: impl NamespaceResolver + Send + Sync + 'static,
        swap_handler: impl SegmentSwapHandler<FS::File>,
    ) -> Result<Self> {
        fs.create_dir_all(&path)?;
        Ok(Self {
            fs,
            path,
            opened: Default::default(),
            shutdown: Default::default(),
            resolver: Box::new(resolver),
            swap_handler: Box::new(swap_handler),
        })
    }

    #[tracing::instrument(skip(self, db_path))]
    pub fn open(self: Arc<Self>, db_path: &Path) -> Result<Arc<SharedWal<FS>>> {
        if self.shutdown.load(Ordering::SeqCst) {
            todo!("open after shutdown");
        }

        let namespace = self.resolver.resolve(db_path);
        loop {
            let mut opened = self.opened.upgradable_read();
            if let Some(entry) = opened.get(&namespace) {
                match entry {
                    Slot::Wal(wal) => return Ok(wal.clone()),
                    Slot::Building(cond) => {
                        let cond = cond.clone();
                        drop(opened);
                        cond.0
                            .wait_while(&mut cond.1.lock(), |ready: &mut bool| !*ready);
                        // the slot was updated: try again
                        continue;
                    }
                }
            }

            // another thread may have got the slot first, just retry if that's the case
            let Ok(notifier) = opened.with_upgraded(|map| match map.entry(namespace.clone()) {
                Entry::Occupied(_) => Err(()),
                Entry::Vacant(entry) => {
                    let notifier = Arc::new((Condvar::new(), Mutex::new(false)));
                    entry.insert(Slot::Building(notifier.clone()));
                    Ok(notifier)
                }
            }) else {
                continue;
            };

            // if try_open succedded, then the slot was updated and contains the shared wal, if it
            // failed we need to remove the slot. Either way, notify all waiters
            let ret = self.clone().try_open(&namespace, db_path, &mut opened);
            if ret.is_err() {
                opened.with_upgraded(|map| {
                    map.remove(&namespace);
                })
            }

            *notifier.1.lock() = true;
            notifier.0.notify_all();

            return ret;
        }
    }

    fn try_open(
        self: Arc<Self>,
        namespace: &NamespaceName,
        db_path: &Path,
        opened: &mut RwLockUpgradableReadGuard<HashMap<NamespaceName, Slot<FS>>>,
    ) -> Result<Arc<SharedWal<FS>>> {
        let path = self.path.join(namespace.as_str());
        self.fs.create_dir_all(&path)?;
        // TODO: handle that with abstract io
        let dir = walkdir::WalkDir::new(&path).sort_by_file_name().into_iter();

        let tail = SegmentList::default();
        for entry in dir {
            let entry = entry.map_err(|e| e.into_io_error().unwrap())?;
            if entry
                .path()
                .extension()
                .map(|e| e.to_str().unwrap() != "seg")
                .unwrap_or(true)
            {
                continue;
            }

            let file = self.fs.open(false, true, true, entry.path())?;

            if let Some(sealed) =
                SealedSegment::open(file.into(), entry.path().to_path_buf(), Default::default())?
            {
                let sealed = Arc::new(sealed);
                self.swap_handler
                    .handle_segment_swap(namespace.clone(), sealed.clone());
                tail.push_log(sealed);
            }
        }

        let db_file = self.fs.open(false, true, true, db_path)?;

        let mut header: Sqlite3DbHeader = Sqlite3DbHeader::new_zeroed();
        db_file.read_exact_at(header.as_bytes_mut(), 0)?;

        let (db_size, next_frame_no) = tail
            .with_head(|segment| {
                let header = segment.header();
                (header.size_after(), header.next_frame_no())
            })
            .unwrap_or((
                header.db_size.get(),
                NonZeroU64::new(header.replication_index.get() + 1)
                    .unwrap_or(NonZeroU64::new(1).unwrap()),
            ));

        let current_path = path.join(format!("{namespace}:{next_frame_no:020}.seg"));

        let segment_file = self.fs.open(true, true, true, &current_path)?;

        let current = arc_swap::ArcSwap::new(Arc::new(CurrentSegment::create(
            segment_file,
            current_path,
            next_frame_no,
            db_size,
            tail.into(),
        )?));

        let (new_frame_notifier, _) = tokio::sync::watch::channel(next_frame_no.get() - 1);

        let shared = Arc::new(SharedWal {
            current,
            wal_lock: Default::default(),
            db_file,
            registry: self.clone(),
            namespace: namespace.clone(),
            checkpointed_frame_no: header.replication_index.get().into(),
            new_frame_notifier,
        });

        opened.with_upgraded(|opened| {
            opened.insert(namespace.clone(), Slot::Wal(shared.clone()));
        });

        return Ok(shared);
    }

    #[tracing::instrument(skip_all)]
    pub fn swap_current(&self, shared: &SharedWal<FS>, tx: &TxGuard<FS::File>) -> Result<()> {
        assert!(tx.is_commited());
        // at this point we must hold a lock to a commited transaction.

        let current = shared.current.load();
        if current.is_empty() {
            return Ok(());
        }
        let start_frame_no = current.next_frame_no();
        let path = self
            .path
            .join(shared.namespace().as_str())
            .join(format!("{}:{start_frame_no:020}.seg", shared.namespace()));

        let segment_file = self.fs.open(true, true, true, &path)?;

        let new = CurrentSegment::create(
            segment_file,
            path,
            start_frame_no,
            current.db_size(),
            current.tail().clone(),
        )?;
        // sealing must the last fallible operation, because we don't want to end up in a situation
        // where the current log is sealed and it wasn't swapped.
        if let Some(sealed) = current.seal()? {
            let sealed = Arc::new(sealed);
            self.swap_handler
                .handle_segment_swap(shared.namespace.clone(), sealed.clone());
            new.tail().push_log(sealed);
        }

        shared.current.swap(Arc::new(new));
        tracing::debug!("current segment swapped");

        Ok(())
    }

    // On shutdown, we checkpoint all the WALs. This require sealing the current segment, and when
    // checkpointing all the segments
    pub fn shutdown(&self) -> Result<()> {
        self.shutdown.store(true, Ordering::SeqCst);
        let mut opened = self.opened.write();
        for (_, shared) in opened.drain() {
            let Slot::Wal(shared) = shared else {
                // TODO: figure out what to do when the wal is being opened
                continue;
            };
            let mut tx = Transaction::Read(shared.begin_read(u64::MAX));
            shared.upgrade(&mut tx)?;
            {
                let mut tx = tx.as_write_mut().unwrap().lock();
                tx.commit();
                self.swap_current(&shared, &tx)?;
            }
            // The current segment will not be used anymore. It's empty, but we still seal it so that
            // the next startup doesn't find an unsealed segment.
            shared.current.load().seal()?;
            drop(tx);
            shared.current.load().tail().checkpoint(&shared.db_file)?;
        }

        Ok(())
    }
}
