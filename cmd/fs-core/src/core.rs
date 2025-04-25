use crate::remote::RemoteMetadataCoordinator;
use anyhow::Context;
use bincode::{deserialize, serialize};
use fuser::{FileAttr, FileType};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::block::BlockDevice;
use crate::layout::*;
// use crate::metadata;
use crate::metadata;

pub struct FsCore {
    pub inode_counter: u64,
    pub inode_attrs: HashMap<u64, FileAttr>,
    pub inode_data: HashMap<u64, Vec<u8>>,
    pub path_to_ino: HashMap<String, u64>,
    pub block_device: BlockDevice,
    pub coordinator: Box<dyn metadata::MetadataCoordinator>,
}

impl FsCore {
    pub fn new(bd: BlockDevice) -> Self {
        let mut fs = FsCore {
            inode_counter: 2, // 1 = root, start from 2
            inode_attrs: HashMap::new(),
            inode_data: HashMap::new(),
            path_to_ino: HashMap::new(),
            block_device: bd,
            coordinator: Box::new(metadata::local::LocalMetadataCoordinator::new()),
        };

        // Add root dir
        let root_attr = FileAttr {
            ino: 1,
            size: 0,
            blocks: 0,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
            blksize: 512,
        };

        fs.inode_attrs.insert(1, root_attr);
        fs.path_to_ino.insert("/".into(), 1);

        // Add /hello.txt
        let _ = fs.create_file("hello.txt", b"hello world\n");
        fs
    }

    pub fn with_coordinator(
        block_device: BlockDevice,
        coordinator: Box<dyn metadata::MetadataCoordinator>,
    ) -> Self {
        Self {
            inode_counter: 1,
            inode_attrs: HashMap::new(),
            inode_data: HashMap::new(),
            path_to_ino: HashMap::new(),
            block_device,
            coordinator,
        }
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
                // device.write_block(block, &padded)?;
                block += 1;
            }
        }
        Ok(())
    }

    pub async fn create_file(&mut self, path: &str, data: &[u8]) -> anyhow::Result<u64> {
        let lock_key = metadata::LockKey(path.to_owned()); // assuming root inode for now
        let timeout = Duration::from_secs(2);

        tracing::trace!("Trying to acquire lock on inode {}", path.to_owned());
        self.coordinator
            .lock(lock_key.clone(), metadata::LockType::Write, timeout)
            .await
            .context("Failed to acquire lock for file creation")?;

        let result = (|| {
            let ino = self.inode_counter;
            self.inode_counter += 1;

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

            Ok(ino)
        })();

        self.coordinator
            .unlock(lock_key)
            .await
            .context("Failed to release lock after file creation")?;

        result
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

    // pub fn load_from_device(&mut self, device: &mut crate::BlockDevice) -> std::io::Result<()> {
    pub fn init(&mut self) -> std::io::Result<()> {
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
        Ok(())
    }
}
