use super::*;
use anyhow;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct LocalMetadataCoordinator {
    locks: Arc<Mutex<HashSet<LockKey>>>,
}

impl LocalMetadataCoordinator {
    pub fn new() -> Self {
        Self {
            locks: Mutex::new(HashSet::new()).into(),
        }
    }
}

#[tonic::async_trait]
impl MetadataCoordinator for LocalMetadataCoordinator {
    async fn lock(
        &self,
        key: LockKey,
        lock_type: LockType,
        timeout: Duration,
    ) -> anyhow::Result<()> {
        let deadline = Instant::now() + timeout;
        loop {
            {
                let mut locks = self.locks.lock().unwrap();
                if !locks.contains(&key) {
                    locks.insert(key.clone());
                    return Ok(());
                }
            }
            if Instant::now() > deadline {
                anyhow::bail!("Timeout while acquiring lock on {:?}", key);
            }
            thread::sleep(Duration::from_millis(10)); // simple backoff
        }
    }

    async fn unlock(&self, key: LockKey) -> anyhow::Result<()> {
        let mut locks = self.locks.lock().unwrap();
        if locks.remove(&key) {
            Ok(())
        } else {
            anyhow::bail!("Tried to unlock a non-held lock {:?}", key);
        }
    }
    
    async fn is_locked(&self, key: &LockKey) -> bool {
        self.locks.lock().unwrap().contains(key)
    }
}
