// src/metadata/remote.rs
use super::*;
use anyhow::{Context, Result};
use proto::metadata::{
    metadata_client::MetadataClient,
    LockRequest, // UnlockRequest, IsLockedRequest,
};
use std::time::Duration;
use tonic::transport::Channel;

#[derive(Clone)]
pub struct RemoteMetadataCoordinator {
    client: MetadataClient<Channel>,
}

impl RemoteMetadataCoordinator {
    pub async fn connect<D: Into<String>>(dst: D) -> Result<Self> {
        let client = MetadataClient::connect(dst.into())
            .await
            .context("Failed to connect to metadata-service")?;
        Ok(Self { client })
    }
}

#[tonic::async_trait]
impl MetadataCoordinator for RemoteMetadataCoordinator {
    async fn lock(
        &self,
        key: LockKey,
        lock_type: LockType,
        timeout: Duration,
    ) -> anyhow::Result<()> {
        let mut client = self.client.clone();
        let req = tonic::Request::new(LockRequest {
            key: key.0,
            // exclusive: matches!(lock_type, LockType::Write),
            // timeout_ms: timeout.as_millis() as u64,
        });

        client.acquire_lock(req).await.context("Lock RPC failed")?;
        Ok(())
    }
    async fn unlock(&self, key: LockKey) -> anyhow::Result<()> {
        Ok(())
    }
    async fn is_locked(&self, key: &LockKey) -> bool {
        true
    }
    // fn unlock(&self, key: LockKey) -> Result<()> {
    //     let mut client = self.client.clone();
    //     let req = tonic::Request::new(UnlockRequest { inode: key.0 });

    //     tokio::runtime::Handle::current().block_on(async move {
    //         client.unlock(req).await.context("Unlock RPC failed")?;
    //         Ok(())
    //     })
    // }

    // fn is_locked(&self, key: &LockKey) -> bool {
    //     let mut client = self.client.clone();
    //     let req = tonic::Request::new(IsLockedRequest { inode: key.0 });

    //     tokio::runtime::Handle::current().block_on(async move {
    //         match client.is_locked(req).await {
    //             Ok(response) => response.into_inner().locked,
    //             Err(_) => false, // fail-safe
    //         }
    //     })
    // }
}
