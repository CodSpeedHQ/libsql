use std::ffi::c_int;
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Arc;

use libsql_sys::rusqlite::{self, ErrorCode, OpenFlags};
use libsql_wal::{
    fs::{file::FileExt, FileSystem, StdFs},
    name::NamespaceName,
    registry::WalRegistry,
    wal::LibsqlWalManager,
};
use parking_lot::Mutex;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use tempfile::tempdir;

#[derive(Clone)]
struct FlakyFs {
    p_failure: f32,
    rng: Arc<Mutex<rand_chacha::ChaCha8Rng>>,
    enabled: Arc<AtomicBool>,
}

struct FlakyFile {
    inner: File,
    fs: FlakyFs,
}

impl FileExt for FlakyFile {
    fn write_all_at(&self, buf: &[u8], offset: u64) -> std::io::Result<()> {
        self.fs
            .with_random_failure(|| self.inner.write_all_at(buf, offset))
    }

    fn write_at_vectored(&self, bufs: &[std::io::IoSlice], offset: u64) -> std::io::Result<usize> {
        self.fs
            .with_random_failure(|| self.inner.write_at_vectored(bufs, offset))
    }

    fn write_at(&self, buf: &[u8], offset: u64) -> std::io::Result<usize> {
        self.fs
            .with_random_failure(|| self.inner.write_at(buf, offset))
    }

    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
        self.fs
            .with_random_failure(|| self.inner.read_exact_at(buf, offset))
    }

    fn sync_all(&self) -> std::io::Result<()> {
        self.fs.with_random_failure(|| self.inner.sync_all())
    }

    fn set_len(&self, len: u64) -> std::io::Result<()> {
        self.fs.with_random_failure(|| self.inner.set_len(len))
    }
}

impl FlakyFs {
    fn with_random_failure<R>(&self, f: impl FnOnce() -> std::io::Result<R>) -> std::io::Result<R> {
        let r = self.rng.lock().gen_range(0.0..1.0);
        if self.enabled.load(Relaxed) && r <= self.p_failure {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "failure"))
        } else {
            f()
        }
    }
}

impl FileSystem for FlakyFs {
    type File = FlakyFile;

    fn create_dir_all(&self, path: &std::path::Path) -> std::io::Result<()> {
        self.with_random_failure(|| StdFs.create_dir_all(path))
    }

    fn open(
        &self,
        create_new: bool,
        read: bool,
        write: bool,
        path: &std::path::Path,
    ) -> std::io::Result<Self::File> {
        self.with_random_failure(|| {
            Ok(FlakyFile {
                inner: StdFs.open(create_new, read, write, path)?,
                fs: self.clone(),
            })
        })
    }
}

macro_rules! assert_not_corrupt {
    ($($e:expr,)*) => {
        $(
            match $e {
                Ok(_) => (),
                Err(e) => {
                    match e.sqlite_error() {
                        Some(e) if e.code == ErrorCode::DatabaseCorrupt => panic!("db corrupt"),
                        _ => ()
                    }
                }
            };
        )*
    };
}

fn enable_libsql_logging() {
    use std::sync::Once;
    static ONCE: Once = Once::new();

    fn libsql_log(code: c_int, msg: &str) {
        println!("sqlite error {code}: {msg}");
    }

    ONCE.call_once(|| unsafe {
        rusqlite::trace::config_log(Some(libsql_log)).unwrap();
    });
}

#[test]
fn flaky_fs() {
    enable_libsql_logging();
    let seed = rand::thread_rng().gen();
    println!("seed: {seed}");
    let enabled = Arc::new(AtomicBool::new(false));
    let fs = FlakyFs {
        p_failure: 0.1,
        rng: Arc::new(Mutex::new(ChaCha8Rng::seed_from_u64(seed))),
        enabled: enabled.clone(),
    };
    let tmp = tempdir().unwrap();
    let resolver = |path: &Path| {
        let name = path.file_name().unwrap().to_str().unwrap();
        NamespaceName::from_string(name.to_string())
    };
    let registry =
        Arc::new(WalRegistry::new_with_fs(fs, tmp.path().join("test/wals"), resolver).unwrap());
    let wal_manager = LibsqlWalManager {
        registry: registry.clone(),
        next_conn_id: Default::default(),
    };

    let conn = libsql_sys::Connection::open(
        tmp.path().join("test/data").clone(),
        OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE,
        wal_manager.clone(),
        100000,
        None,
    )
    .unwrap();

    let _ = conn.execute(
        "CREATE TABLE t1(a INTEGER PRIMARY KEY, b BLOB(16), c BLOB(16), d BLOB(400));",
        (),
    );
    let _ = conn.execute("CREATE INDEX i1 ON t1(b);", ());
    let _ = conn.execute("CREATE INDEX i2 ON t1(c);", ());

    enabled.store(true, Relaxed);

    for _ in 0..50_000 {
        assert_not_corrupt! {
            conn.execute("REPLACE INTO t1 VALUES(abs(random() % 5000000), randomblob(16), randomblob(16), randomblob(400));", ()),
            conn.execute("REPLACE INTO t1 VALUES(abs(random() % 5000000), randomblob(16), randomblob(16), randomblob(400));", ()),
            conn.execute("REPLACE INTO t1 VALUES(abs(random() % 5000000), randomblob(16), randomblob(16), randomblob(400));", ()),
        }

        let mut stmt = conn
            .prepare("SELECT * FROM t1 WHERE a>abs((random()%5000000)) LIMIT 10;")
            .unwrap();

        assert_not_corrupt! {
            stmt.query(()).map(|r| r.mapped(|_r| Ok(())).count()),
            stmt.query(()).map(|r| r.mapped(|_r| Ok(())).count()),
            stmt.query(()).map(|r| r.mapped(|_r| Ok(())).count()),
        }
    }

    enabled.store(false, Relaxed);

    conn.pragma_query(None, "integrity_check", |r| {
        dbg!(r);
        Ok(())
    })
    .unwrap();
    conn.query_row("select count(0) from t1", (), |r| {
        dbg!(r);
        Ok(())
    })
    .unwrap();
}
