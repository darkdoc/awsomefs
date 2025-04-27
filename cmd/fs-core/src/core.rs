use anyhow::Context;
use bincode::{deserialize, serialize};

use std::sync::Arc;
use tokio::sync::Mutex; // Use tokio::sync::Mutex

use fuser::{FileAttr, FileType};
use metadata::MetadataCoordinator;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::block::BlockDevice;
use crate::layout::*;
use crate::metadata;

pub const ROOT_INO: u64 = 1;

pub struct FsCoreInner {
    pub inode_counter: u64,
    pub inode_attrs: HashMap<u64, FileAttr>,
    pub inode_data: HashMap<u64, Vec<u8>>,
    pub path_to_ino: HashMap<String, u64>,
    pub block_device: BlockDevice,
    pub parent_to_children: HashMap<u64, HashMap<String, u64>>,
}

impl FsCoreInner {
    pub fn new(block_device: BlockDevice) -> Self {
        let mut inner = FsCoreInner {
            block_device,
            inode_counter: 2, // Reserve 1 for ROOT_INO
            inode_attrs: HashMap::new(),
            inode_data: HashMap::new(),
            path_to_ino: HashMap::new(),
            parent_to_children: HashMap::new(),
        };

        // Insert root directory if not already there
        inner.insert_root_dir();

        inner
    }

    pub fn create_file_locked(
        &mut self,
        parent_ino: u64,
        name: &str,
        data: &[u8],
    ) -> anyhow::Result<u64> {
        let ino = self.inode_counter;
        self.inode_counter += 1;

        let path = if parent_ino == ROOT_INO {
            format!("/{}", name)
        } else {
            unimplemented!("nested dirs not yet supported"); // for now
        };

        let attr = FileAttr {
            ino,
            size: data.len() as u64,
            blocks: 1,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: FileType::RegularFile,
            perm: 0o644,
            nlink: 1,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
            blksize: 512,
        };

        self.inode_attrs.insert(ino, attr);
        self.inode_data.insert(ino, data.to_vec());
        self.path_to_ino.insert(path.to_string(), ino);

        self.parent_to_children
            .entry(parent_ino)
            .or_default()
            .insert(name.to_string(), ino);

        Ok(ino)
    }

    pub fn load_from_device(&mut self) -> std::io::Result<()> {
        let mut block = 1;
        loop {
            let mut buf = vec![0u8; self.block_device.block_size];
            match self.block_device.read_block(block, &mut buf) {
                Ok(_) => {
                    match deserialize::<PersistedInode>(&buf) {
                        Ok(serialized_inode) => {
                            let inode = serialized_inode;
                            let ino = inode.attr.ino;
                            self.inode_attrs.insert(ino, inode.attr.into());
                            self.inode_data.insert(ino, inode.data);
                            self.path_to_ino.insert(inode.path, ino);
                            self.inode_counter = self.inode_counter.max(ino + 1);
                        }
                        Err(_) => break, // Assume we hit uninitialized block
                    }
                }
                Err(_) => break,
            }
            block += 1;
        }

        // After loading, ensure root exists
        if !self.inode_attrs.contains_key(&ROOT_INO) {
            self.insert_root_dir();
        }
        Ok(())
    }

    pub fn save_to_device(&mut self) -> std::io::Result<()> {
        let mut block = 1;
        for (ino, attr) in &self.inode_attrs {
            if let Some(data) = self.inode_data.get(ino) {
                let path = self
                    .path_to_ino
                    .iter()
                    .find(|(_, v)| *v == ino)
                    .map(|(k, _)| k.clone())
                    .unwrap_or_else(|| "".to_string());

                let inode = PersistedInode {
                    attr: (*attr).clone().into(), // Convert to SerializableFileAttr
                    data: data.clone(),
                    path,
                };

                let bytes = serialize(&inode).unwrap();
                let mut padded = vec![0u8; self.block_device.block_size];
                padded[..bytes.len()].copy_from_slice(&bytes);
                self.block_device.write_block(block, &padded)?;
                block += 1;
            }
        }
        Ok(())
    }

    pub fn write_and_persist_inode(&mut self, ino: u64, offset: u64, data: &[u8]) -> usize {
        let file_data = self.inode_data.entry(ino).or_insert_with(Vec::new);

        let end_offset = (offset as usize) + data.len();
        if file_data.len() < end_offset {
            file_data.resize(end_offset, 0);
        }

        file_data[offset as usize..end_offset].copy_from_slice(data);

        if let Some(attr) = self.inode_attrs.get_mut(&ino) {
            attr.size = file_data.len() as u64;
        }

        // 💾 Persist change to device in a scoped block to avoid borrow checker issues

        if let Err(e) = self.save_to_device() {
            log::error!("Failed to persist inode {} to device: {}", ino, e);
        }

        data.len()
    }

    pub fn insert_root_dir(&mut self) {
        let attr = FileAttr {
            ino: ROOT_INO,
            size: 0,
            blocks: 0,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2, // '.' and '..'
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
            blksize: 512,
        };

        self.inode_attrs.insert(ROOT_INO, attr);
        self.inode_data.insert(ROOT_INO, Vec::new()); // empty directory data
        self.path_to_ino.insert("/".to_string(), ROOT_INO);
    }
}

pub struct FsCore {
    inner: Arc<Mutex<FsCoreInner>>,
    pub coordinator: Box<dyn MetadataCoordinator>,
}

impl FsCore {
    pub fn new(block_device: BlockDevice) -> Arc<Self> {
        Arc::new(FsCore {
            inner: Arc::new(Mutex::new(FsCoreInner::new(block_device))),
            coordinator: Box::new(metadata::local::LocalMetadataCoordinator::new()),
        })
    }

    pub fn with_coordinator(
        block_device: BlockDevice,
        coordinator: Box<dyn MetadataCoordinator>,
    ) -> Arc<Self> {
        Arc::new(FsCore {
            inner: Arc::new(Mutex::new(FsCoreInner::new(block_device))),
            coordinator,
        })
    }
    pub async fn load_from_device(&self) -> std::io::Result<()> {
        self.with_inner(|inner| inner.load_from_device()).await
    }

    pub async fn create_file(
        &self,
        parent_ino: u64,
        name: &str,
        data: &[u8],
    ) -> anyhow::Result<u64> {
        let lock_key = metadata::LockKey(parent_ino);
        let timeout: Duration = Duration::from_secs(2);

        tracing::trace!("Trying to acquire lock on inode {}", parent_ino);
        self.coordinator
            .lock(lock_key.clone(), metadata::LockType::Write, timeout)
            .await
            .context("Failed to acquire lock for file creation")?;

        let result = {
            let mut fs = self.inner.lock().await;
            fs.create_file_locked(parent_ino, name, data)
        };
        self.coordinator
            .unlock(lock_key)
            .await
            .context("Failed to release lock after file creation")?;

        result
    }

    pub async fn init_async(&self) -> std::io::Result<()> {
        let mut inner = self.inner.lock().await;
        inner.load_from_device()?;
        Ok(())
    }

    pub async fn with_inner<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut FsCoreInner) -> R,
    {
        let mut inner = self.inner.lock().await;
        f(&mut inner)
    }

    pub async fn with_inner_result<F, R, E>(&self, f: F) -> Result<R, E>
    where
        F: FnOnce(&mut FsCoreInner) -> Result<R, E>,
    {
        let mut inner = self.inner.lock().await;
        f(&mut inner)
    }

    pub fn blocking_lock_inner(&self) -> tokio::sync::MutexGuard<'_, FsCoreInner> {
        // blocking_lock() works for cases like readdir
        self.inner.blocking_lock()
    }
}
