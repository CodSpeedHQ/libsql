use std::marker::PhantomData;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use libsql_sys::name::NamespaceName;

use crate::io::FileExt;
use crate::segment::{sealed::SealedSegment, Segment};

pub use self::error::Error;

mod job;
// mod restore;
pub mod async_storage;
pub mod backend;
pub(crate) mod error;
mod scheduler;

pub type Result<T, E = self::error::Error> = std::result::Result<T, E>;

pub enum RestoreOptions {
    Latest,
    Timestamp(DateTime<Utc>),
}

pub trait Storage: Send + Sync + 'static {
    type Segment: Segment;
    type Config;
    /// store the passed segment for `namespace`. This function is called in a context where
    /// blocking is acceptable.
    fn store(
        &self,
        namespace: &NamespaceName,
        seg: Self::Segment,
        config_override: Option<Arc<Self::Config>>,
    );

    async fn durable_frame_no(
        &self,
        namespace: &NamespaceName,
        config_override: Option<Arc<Self::Config>>,
    ) -> u64;

    async fn restore(
        &self,
        file: impl FileExt,
        namespace: &NamespaceName,
        restore_options: RestoreOptions,
        config_override: Option<Arc<Self::Config>>,
    ) -> Result<()>;
}

/// a placeholder storage that doesn't store segment
#[derive(Debug, Clone, Copy)]
pub struct NoStorage;

impl Storage for NoStorage {
    type Config = ();
    type Segment = SealedSegment<std::fs::File>;

    fn store(
        &self,
        _namespace: &NamespaceName,
        _seg: Self::Segment,
        _config: Option<Arc<Self::Config>>,
    ) {
    }

    async fn durable_frame_no(
        &self,
        _namespace: &NamespaceName,
        _config: Option<Arc<Self::Config>>,
    ) -> u64 {
        u64::MAX
    }

    async fn restore(
        &self,
        _file: impl FileExt,
        _namespace: &NamespaceName,
        _restore_options: RestoreOptions,
        _config_override: Option<Arc<Self::Config>>,
    ) -> Result<()> {
        panic!("can restore from no storage")
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct TestStorage<F = std::fs::File>(PhantomData<F>);

impl<F> Clone for TestStorage<F> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<F: FileExt> TestStorage<F> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<F: FileExt + Send + Sync + 'static> Storage for TestStorage<F> {
    type Segment = SealedSegment<F>;
    type Config = ();

    fn store(
        &self,
        _namespace: &NamespaceName,
        _seg: Self::Segment,
        _config: Option<Arc<Self::Config>>,
    ) {
    }

    async fn durable_frame_no(
        &self,
        _namespace: &NamespaceName,
        _config: Option<Arc<Self::Config>>,
    ) -> u64 {
        u64::MAX
    }

    async fn restore(
        &self,
        _file: impl FileExt,
        _namespace: &NamespaceName,
        _restore_options: RestoreOptions,
        _config_override: Option<Arc<Self::Config>>,
    ) -> Result<()> {
        todo!();
    }
}

#[derive(Debug)]
pub struct StoreSegmentRequest<C, S> {
    namespace: NamespaceName,
    /// Path to the segment. Read-only for bottomless
    segment: S,
    /// When this segment was created
    created_at: DateTime<Utc>,

    /// alternative configuration to use with the storage layer.
    /// e.g: S3 overrides
    storage_config_override: Option<Arc<C>>,
}
