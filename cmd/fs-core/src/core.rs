use anyhow::Context;
use bincode::{deserialize, serialize};

use std::sync::Arc;
use tokio::sync::Mutex;

use fuser::{FileAttr, FileType};
use metadata::MetadataCoordinator;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::block::BlockDevice;
use crate::layout::*;
use crate::metadata;
use crate::Superblock;

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
    pub fn new(mut block_device: BlockDevice) -> Self {
        let inode_counter = {
            let superblock = Superblock::load(&mut block_device.file, block_device.block_size)
                .expect("Failed to load superblock");
            superblock.inode_count
        };

        FsCoreInner {
            inode_counter,
            inode_attrs: HashMap::new(),
            inode_data: HashMap::new(),
            path_to_ino: HashMap::new(),
            block_device,
            parent_to_children: HashMap::new(),
        }
    }

    pub fn load_superblock(&mut self) -> std::io::Result<()> {
        let mut file = &mut self.block_device.file;
        let block_size = self.block_device.block_size;
        let sb = Superblock::load(&mut file, block_size)?;
        self.inode_counter = sb.inode_count;
        Ok(())
    }

    pub fn save_superblock(&mut self) -> std::io::Result<()> {
        let mut file = &mut self.block_device.file;
        let block_size = self.block_device.block_size;

        let mut superblock = Superblock::load(&mut file, block_size)?;
        superblock.inode_count = self.inode_counter;
        superblock.save(&mut file, block_size)
    }

    pub fn create_file_locked(
        &mut self,
        parent_ino: u64,
        name: &str,
        data: &[u8],
    ) -> anyhow::Result<u64> {
        self.load_superblock()?;

        self.inode_counter += 1;
        let ino = self.inode_counter;

        let path = if parent_ino == ROOT_INO {
            format!("/{}", name)
        } else {
            unimplemented!("nested dirs not yet supported");
        };

        // Create attributes for the new file
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

        // Update in-memory structures
        self.inode_attrs.insert(ino, attr);
        self.inode_data.insert(ino, data.to_vec());
        self.path_to_ino.insert(path.clone(), ino);
        self.parent_to_children
            .entry(parent_ino)
            .or_default()
            .insert(name.to_string(), ino);

        // Persist the new file's inode
        let file_inode = PersistedInode {
            attr: attr.into(),   // Convert to SerializableFileAttr
            data: data.to_vec(), // File contents
            path: path.clone(),
        };
        self.save_inode(ino, &file_inode)?;

        // Regenerate and persist the parent directory's data
        if let Some(children) = self.parent_to_children.get(&parent_ino) {
            let entries: Vec<DirectoryEntry> = children
                .iter()
                .map(|(name, ino)| DirectoryEntry {
                    name: name.clone(),
                    ino: *ino,
                })
                .collect();

            let serialized = bincode::serialize(&entries).unwrap();

            let parent_path = self
                .path_to_ino
                .iter()
                .find(|(_, &ino)| ino == parent_ino)
                .map(|(p, _)| p.clone())
                .unwrap();

            let parent_inode = PersistedInode {
                attr: self.inode_attrs.get(&parent_ino).unwrap().clone().into(),
                data: serialized,
                path: parent_path.clone(),
            };
            self.save_inode(parent_ino, &parent_inode)?;
        }
        self.save_superblock()?;
        Ok(ino)
    }

    pub fn load_from_device(&mut self) -> std::io::Result<()> {
        let mut ino = 1;
        loop {
            match self.load_inode(ino) {
                Ok(inode) => {
                    let attr = inode.attr.clone();
                    self.inode_attrs.insert(ino, attr.into());
                    self.inode_data.insert(ino, inode.data.clone());
                    self.path_to_ino.insert(inode.path, ino);
                    self.inode_counter = self.inode_counter.max(ino + 1);

                    if attr.kind == fuser::FileType::Directory {
                        if let Ok(entries) =
                            bincode::deserialize::<Vec<DirectoryEntry>>(&inode.data)
                        {
                            tracing::debug!("Loaded directory {} entries: {:?}", ino, entries);

                            let mut map = HashMap::new();
                            for entry in entries {
                                map.insert(entry.name, entry.ino);
                            }
                            self.parent_to_children.insert(ino, map);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                    // Likely uninitialized block or invalid data, stop scanning
                    break;
                }
                Err(e) => {
                    // Any other I/O error, also break or handle differently if needed
                    tracing::warn!("Error loading inode {}: {}", ino, e);
                    break;
                }
            }

            ino += 1;
        }

        // Ensure root inode is present
        if !self.inode_attrs.contains_key(&ROOT_INO) {
            self.insert_root_dir();
        }

        Ok(())
    }

    pub fn get_or_load_inode(&mut self, ino: u64) -> std::io::Result<PersistedInode> {
        // TODO some caching, needs fixing to work with multi-mount
        // if let Some(attr) = self.inode_attrs.get(&ino) {
        //     if let Some(data) = self.inode_data.get(&ino) {
        //         if let Some(path) = self.path_to_ino.iter().find_map(|(path, i)| {
        //             if *i == ino {
        //                 Some(path.clone())
        //             } else {
        //                 None
        //             }
        //         }) {
        //             return Ok(PersistedInode {
        //                 attr: attr.clone().into(),
        //                 data: data.clone(),
        //                 path,
        //             });
        //         }
        //     }
        // }

        // Fallback to disk
        let inode = self.load_inode(ino)?;
        self.inode_attrs.insert(ino, inode.attr.clone().into());
        self.inode_data.insert(ino, inode.data.clone());
        self.path_to_ino.insert(inode.path.clone(), ino);
        Ok(inode)
    }

    pub fn load_inode(&mut self, ino: u64) -> std::io::Result<PersistedInode> {
        let block = 1 + ino; // Block 0 is superblock
        let mut buf = vec![0u8; self.block_device.block_size];
        self.block_device.read_block(block, &mut buf)?;

        let len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;

        if len == 0 || 4 + len > buf.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid inode length",
            ));
        }

        let serialized = &buf[4..4 + len];

        bincode::deserialize::<PersistedInode>(serialized)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn save_inode(&mut self, ino: u64, inode: &PersistedInode) -> std::io::Result<()> {
        let block = 1 + ino; // Block 0 is superblock

        let bytes = serialize(&inode).unwrap();
        let len = bytes.len() as u32; // 4 bytes to store size
        let mut padded = vec![0u8; self.block_device.block_size];

        if 4 + bytes.len() > self.block_device.block_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Serialized inode too large for block",
            ));
        }

        padded[..4].copy_from_slice(&len.to_le_bytes()); // Save length first
        padded[4..4 + bytes.len()].copy_from_slice(&bytes);

        self.block_device.write_block(block, &padded)
    }

    pub fn unlink_locked(&mut self, parent_ino: u64, name: &str) -> anyhow::Result<()> {
        unimplemented!("nested dirs not yet fully supported yet");

        // Remove the file from the parent directory
        // if let Some(parent) = self.parent_to_children.get_mut(&parent_ino) {
        //     if let Some(ino) = parent.remove(name) {
        //         // 1. Remove the inode data (file attributes and data)
        //         self.inode_attrs.remove(&ino);
        //         self.inode_data.remove(&ino);

        //         // 2. Remove the file path reference
        //         self.path_to_ino.retain(|_, v| *v != ino);

        //         // 3. Remove the inode from disk
        //         self.delete_inode_from_disk(ino)?;

        //         // Success
        //         Ok(())
        //     } else {
        //         Err(anyhow::anyhow!("File not found in directory").into())
        //     }
        // } else {
        //     Err(anyhow::anyhow!("Parent directory not found").into())
        // }
    }

    fn delete_inode_from_disk(&mut self, ino: u64) -> std::io::Result<()> {
        // Handle removing the inode from storage (from disk)
        let block = 1 + ino; // Adjust based on your block structure, for example

        let mut buf = vec![0u8; self.block_device.block_size];
        buf.fill(0); // Empty the inode

        self.block_device.write_block(block, &buf)
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

        let entries: Vec<DirectoryEntry> = Vec::new();
        let serialized_entries = bincode::serialize(&entries).unwrap();

        let root_inode = PersistedInode {
            attr: attr.into(), // Convert to SerializableFileAttr
            data: serialized_entries.clone(),
            path: "/".to_string(),
        };

        self.save_inode(ROOT_INO, &root_inode).unwrap();
        self.inode_attrs.insert(ROOT_INO, attr);
        self.inode_data.insert(ROOT_INO, Vec::new()); // empty directory data
        self.path_to_ino.insert("/".to_string(), ROOT_INO);
    }

    pub fn mkdir(
        &mut self,
        parent_ino: u64,
        name: &str,
        uid: u32,
        gid: u32,
    ) -> std::io::Result<FileAttr> {
        self.load_superblock()?;

        self.inode_counter += 1;
        let ino = self.inode_counter;

        let attr = FileAttr {
            ino,
            size: 0,
            blocks: 0,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid,
            gid,
            rdev: 0,
            flags: 0,
            blksize: 512,
        };

        let path = if parent_ino == ROOT_INO {
            format!("/{}", name)
        } else {
            // In real FS: lookup parent path and concatenate
            unimplemented!("nested dirs not yet fully supported yet");
        };

        // Persist the new director's inode
        let entries: Vec<DirectoryEntry> = Vec::new();
        let serialized_entries = bincode::serialize(&entries).unwrap();

        let dir_inode = PersistedInode {
            attr: attr.into(), // Convert to SerializableFileAttr
            data: serialized_entries.clone(),
            path: path.to_string(),
        };
        self.save_inode(ino, &dir_inode).unwrap();

        // Update in-memory structures
        self.inode_attrs.insert(ino, attr);
        self.inode_data.insert(ino, Vec::new()); // empty directory data
        self.path_to_ino.insert(path, ino);

        self.parent_to_children
            .entry(parent_ino)
            .or_default()
            .insert(name.to_string(), ino);

        // Regenerate and persist the parent directory's data
        if let Some(children) = self.parent_to_children.get(&parent_ino) {
            let entries: Vec<DirectoryEntry> = children
                .iter()
                .map(|(name, ino)| DirectoryEntry {
                    name: name.clone(),
                    ino: *ino,
                })
                .collect();

            let serialized = bincode::serialize(&entries).unwrap();

            let parent_path = self
                .path_to_ino
                .iter()
                .find(|(_, &ino)| ino == parent_ino)
                .map(|(p, _)| p.clone())
                .unwrap();

            let parent_inode = PersistedInode {
                attr: self.inode_attrs.get(&parent_ino).unwrap().clone().into(),
                data: serialized,
                path: parent_path.clone(),
            };
            self.save_inode(parent_ino, &parent_inode)?;
        }
        self.save_superblock()?;
        Ok(attr)
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
            let result = fs.create_file_locked(parent_ino, name, data);

            result
        };

        self.coordinator
            .unlock(lock_key)
            .await
            .context("Failed to release lock after file creation")?;

        result
    }

    pub async fn unlink(&self, parent_ino: u64, name: &str) -> anyhow::Result<()> {
        self.with_inner(|inner| inner.unlink_locked(parent_ino, name))
            .await
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
