use fuser::{
    FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen, Request,
    TimeOrNow,
};

use libc::{EIO, ENOENT};
use std::ffi::OsStr;
use std::sync::Arc;

use crate::layout::*;

use std::time::{Duration, SystemTime};

const TTL: Duration = Duration::from_secs(1); // 1 second

pub struct AwsomeFs {
    core: Arc<crate::FsCore>,
}

impl AwsomeFs {
    pub async fn new(core: Arc<crate::FsCore>) -> std::io::Result<Self> {
        core.load_from_device().await?; // <-- load early!
        tracing::info!("Filesystem loaded");
        Ok(Self { core })
    }
}

impl Filesystem for AwsomeFs {
    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name = name.to_owned();
        let core = self.core.clone();

        tokio::spawn(async move {
            core.with_inner(|inner| {
                inner.load_superblock().unwrap();

                let parent_path = inner
                    .path_to_ino
                    .iter()
                    .find(|(_, &ino)| ino == parent)
                    .map(|(p, _)| p.clone())
                    .unwrap();

                let path = if parent == crate::ROOT_INO {
                    format!("/{}", name.to_str().unwrap())
                } else {
                    format!("{}/{}", parent_path, name.to_str().unwrap())
                };

                for ino in 1..inner.inode_counter + 1 {
                    if let Ok(inode) = inner.get_or_load_inode(ino) {
                        if inode.path == path {
                            reply.entry(&TTL, &inode.attr.into(), 0);
                            return;
                        }
                    }
                }
                tracing::info!("this is where its wrong {} for", path);

                reply.error(ENOENT);
            })
            .await;
        });
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        let core = self.core.clone();

        tokio::task::block_in_place(|| {
            let mut inner = core.blocking_lock_inner();

            match inner.get_or_load_inode(ino) {
                Ok(inode) => {
                    reply.attr(&TTL, &inode.attr.into());
                }
                Err(_) => {
                    tracing::info!("getattr: inode {} not found", ino);
                    reply.error(ENOENT);
                }
            }
        });
    }

    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        let name = name.to_string_lossy().to_string();
        let core = self.core.clone(); // Arc<FsCore>

        tokio::task::block_in_place(|| {
            let mut inner = core.blocking_lock_inner();

            match inner.mkdir(parent, &name, 1000, 1000) {
                // TODO: real uid/gid
                Ok(attr) => {
                    reply.entry(&TTL, &attr, 0);
                }
                Err(_) => {
                    reply.error(libc::EIO);
                }
            }
        });
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        let core = self.core.clone();

        tokio::spawn(async move {
            core.with_inner(|inner| match inner.load_inode(ino) {
                Ok(inode) => {
                    let start = offset as usize;
                    let end = (start + size as usize).min(inode.data.len());

                    if start >= inode.data.len() {
                        reply.data(&[]);
                    } else {
                        reply.data(&inode.data[start..end]);
                    }
                }
                Err(_) => reply.error(ENOENT),
            })
            .await;
        });
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let core = self.core.clone();

        tokio::task::block_in_place(|| {
            let mut inner = core.blocking_lock_inner();

            let mut entries = vec![
                (ino, FileType::Directory, ".".into()),
                (crate::ROOT_INO, FileType::Directory, "..".into()),
            ];

            match inner.get_or_load_inode(ino) {
                Ok(inode) => {
                    let dir_entries: Vec<DirectoryEntry> = if inode.data.is_empty()
                        || inode.attr.kind != FileType::Directory
                    {
                        Vec::new()
                    } else {
                        match bincode::deserialize::<Vec<DirectoryEntry>>(&inode.data) {
                            Ok(entries) => entries,
                            Err(e) => {
                                tracing::error!("Failed to deserialize directory entries: {}", e);
                                Vec::new()
                            }
                        }
                    };

                    for entry in dir_entries {
                        let file_type = match inner.get_or_load_inode(entry.ino) {
                            Ok(child_inode) => child_inode.attr.kind.into(),
                            Err(_) => FileType::RegularFile, // fallback
                        };

                        entries.push((entry.ino, file_type, entry.name));
                    }
                }
                Err(_) => {
                    reply.error(ENOENT);
                    return;
                }
            }

            for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
                if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                    break;
                }
            }

            reply.ok();
        });
    }

    // fn readdir(
    //     &mut self,
    //     _req: &Request<'_>,
    //     ino: u64,
    //     _fh: u64,
    //     offset: i64,
    //     mut reply: ReplyDirectory,
    // ) {
    //     let core = self.core.clone();

    //     tokio::task::block_in_place(|| {
    //         let entries = {
    //             let mut inner = core.blocking_lock_inner();

    //             let mut entries = vec![
    //                 (ino, FileType::Directory, ".".into()),
    //                 (crate::ROOT_INO, FileType::Directory, "..".into()),
    //             ];

    //             match inner.load_inode(ino) {
    //                 Ok(inode) => {
    //                     tracing::error!("readdir Loaded inode: {:?}", inode);
    //                     let dir_entries: Vec<DirectoryEntry> = if inode.data.is_empty()
    //                         || inode.attr.kind != fuser::FileType::Directory
    //                     {
    //                         Vec::new()
    //                     } else {
    //                         bincode::deserialize::<Vec<DirectoryEntry>>(&inode.data).unwrap()
    //                         // bincode::deserialize(&inode.data).unwrap_or_default()
    //                     };

    //                     for entry in dir_entries {
    //                         let file_type = if let Some(attr) = inner.inode_attrs.get(&entry.ino) {
    //                             attr.kind
    //                         } else if let Ok(child_inode) = inner.load_inode(entry.ino) {
    //                             let kind = child_inode.attr.kind;
    //                             let child_inode = child_inode.clone();
    //                             // Cache it into memory for future use
    //                             inner.inode_attrs.insert(entry.ino, child_inode.attr.into());
    //                             inner.inode_data.insert(entry.ino, child_inode.data);
    //                             kind.into()
    //                         } else {
    //                             // Assume regular file if unknown
    //                             // SerializableFileType::RegularFile.into()
    //                             FileType::RegularFile
    //                         };

    //                         entries.push((entry.ino, file_type, entry.name.clone()));
    //                     }
    //                 }
    //                 Err(_) => {
    //                     reply.error(ENOENT);
    //                     return;
    //                 }
    //             }

    //             entries
    //         };

    //         for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
    //             if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
    //                 break;
    //             }
    //         }

    //         reply.ok();
    //     });
    // }

    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: fuser::ReplyEmpty) {
        let core = self.core.clone();
        let name = name.to_str().unwrap_or("").to_string();

        tokio::spawn(async move {
            let result = core.unlink(parent, &name).await;

            match result {
                Ok(_) => reply.ok(),
                Err(_) => reply.error(libc::ENOENT),
            }
        });
    }
    fn open(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        let core = self.core.clone(); // you'll need Arc<Mutex<FsCore>>
        tokio::spawn(async move {
            core.with_inner(|inner| {
                match inner.load_inode(ino) {
                    Ok(_) => {
                        // Successfully found inode on disk
                        reply.opened(0, 0);
                    }
                    Err(_) => {
                        reply.error(libc::ENOENT);
                    }
                }
            })
            .await;
        });
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        let core = self.core.clone();
        let data = data.to_vec(); // <-- clone the slice into an owned Vec

        tokio::spawn(async move {
            core.with_inner(|inner| match inner.load_inode(ino) {
                Ok(mut inode) => {
                    let start = offset as usize;
                    let end = start + data.len();

                    if inode.data.len() < end {
                        inode.data.resize(end, 0);
                    }

                    inode.data[start..end].copy_from_slice(&data);
                    inode.attr.size = inode.data.len() as u64;

                    if let Err(_) = inner.save_inode(ino, &inode) {
                        reply.error(EIO);
                    } else {
                        reply.written(data.len() as u32);
                    }
                }
                Err(_) => reply.error(ENOENT),
            })
            .await;
        });
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        let core = self.core.clone();
        let name = name.to_str().unwrap_or("").to_string();

        tokio::spawn(async move {
            let ino = match core.create_file(parent, &name, &[]).await {
                Ok(ino) => ino,
                Err(_) => {
                    tracing::error!("create_file failed, parent:{} name:{}", parent, name);

                    reply.error(EIO);
                    return;
                }
            };

            core.with_inner(|inner| match inner.load_inode(ino) {
                Ok(inode) => {
                    reply.created(&TTL, &inode.attr.into(), 0, 0, 0);
                }
                Err(_) => {
                    tracing::error!("Missing inode after creation, ino={}", ino);
                    reply.error(ENOENT);
                }
            })
            .await;
        });
    }

    fn setattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<fuser::TimeOrNow>,
        mtime: Option<fuser::TimeOrNow>,
        ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        let core = self.core.clone();
        tokio::spawn(async move {
            core.with_inner(|inner| {
                match inner.load_inode(ino) {
                    Ok(mut inode) => {
                        if let Some(new_size) = size {
                            inode.data.resize(new_size as usize, 0);
                            inode.attr.size = new_size;
                        }

                        if let Some(new_uid) = uid {
                            inode.attr.uid = new_uid;
                        }
                        if let Some(new_gid) = gid {
                            inode.attr.gid = new_gid;
                        }
                        if let Some(new_mtime) = mtime {
                            inode.attr.mtime = match new_mtime {
                                TimeOrNow::SpecificTime(t) => t,
                                TimeOrNow::Now => SystemTime::now(),
                            };
                        }
                        if let Some(new_atime) = atime {
                            inode.attr.atime = match new_atime {
                                TimeOrNow::SpecificTime(t) => t,
                                TimeOrNow::Now => SystemTime::now(),
                            };
                        }
                        if let Some(new_ctime) = ctime {
                            inode.attr.ctime = new_ctime;
                        }

                        // Save modified inode back to disk
                        if let Err(e) = inner.save_inode(ino, &inode) {
                            tracing::error!("Failed to save inode after setattr: {:?}", e);
                            reply.error(libc::EIO);
                            return;
                        }

                        // Reply with updated attributes
                        reply.attr(&TTL, &inode.attr.into());
                    }
                    Err(_) => {
                        reply.error(libc::ENOENT);
                    }
                }
            })
            .await;
        });
    }
    // Implement more methods: readdir, read, write, etc.
}
