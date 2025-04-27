use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};


use proto::metadata::{
    metadata_server::{Metadata, MetadataServer},
    LockRequest, LockResponse,
};

#[derive(Debug, Default)]
pub struct MetadataService {
    locked_keys: Arc<Mutex<HashSet<u64>>>,
}

#[tonic::async_trait]
impl Metadata for MetadataService {
    async fn acquire_lock(
        &self,
        request: Request<LockRequest>,
    ) -> Result<Response<LockResponse>, Status> {
        let key = request.into_inner().key;

        let mut locks = self.locked_keys.lock().await;
        if locks.contains(&key) {
            return Ok(Response::new(LockResponse {
                success: false,
                message: format!("Key '{}' is already locked", key),
            }));
        }

        locks.insert(key.clone());

        Ok(Response::new(LockResponse {
            success: true,
            message: format!("Lock acquired for '{}'", key),
        }))
    }

    async fn release_lock(
        &self,
        request: Request<LockRequest>,
    ) -> Result<Response<LockResponse>, Status> {
        let key = request.into_inner().key;

        let mut locks = self.locked_keys.lock().await;
        let removed = locks.remove(&key);

        Ok(Response::new(LockResponse {
            success: removed,
            message: if removed {
                format!("Lock released for '{}'", key)
            } else {
                format!("No lock held for '{}'", key)
            },
        }))
    }
}

pub fn build_metadata_server() -> MetadataServer<MetadataService> {
    MetadataServer::new(MetadataService::default())
}
