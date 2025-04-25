// use std::collections::HashMap;
// use std::path::PathBuf;
use std::time::Duration;
// use anyhow::Result;
// use std::result::Result;

pub mod local;
pub mod remote;


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LockKey(pub String); // will be inode ID, inode path for now

#[derive(Debug)]
pub enum LockType {
    Read,
    Write,
}

#[tonic::async_trait]
pub trait MetadataCoordinator: Send + Sync {
    async fn lock(&self, key: LockKey, lock_type: LockType, timeout: Duration) -> anyhow::Result<()>;
    async fn unlock(&self, key: LockKey) -> anyhow::Result<()>;
    async fn is_locked(&self, key: &LockKey) -> bool;
}
