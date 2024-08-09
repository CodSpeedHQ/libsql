use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::connection::libsql::{LibsqlConnection, MakeLibsqlConnection};
use crate::connection::{MakeThrottledConnection, TrackedConnection};

pub type LibsqlPrimaryConnection = TrackedConnection<LibsqlConnection>;
pub type LibsqlPrimaryConnectionMaker = MakeThrottledConnection<MakeLibsqlConnection>;

pub struct LibsqlPrimaryDatabase {
    pub connection_maker: Arc<LibsqlPrimaryConnectionMaker>,
    pub block_writes: Arc<AtomicBool>,
}

impl LibsqlPrimaryDatabase {
    pub fn connection_maker(&self) -> Arc<LibsqlPrimaryConnectionMaker> {
        self.connection_maker.clone()
    }

    pub fn destroy(self) { }

    pub async fn shutdown(self) -> anyhow::Result<()> { Ok(()) }
}
