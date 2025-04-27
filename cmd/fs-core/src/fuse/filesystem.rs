use fuser::{
    FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen, Request,
    TimeOrNow,
};

use libc::{EIO, ENOENT};
use std::ffi::OsStr;
use std::sync::Arc;

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
    fn lookup(&mut self, _req: &Request<'_>, _parent: u64, name: &OsStr, reply: ReplyEntry) {
        // Stub
        let path = format!("/{}", name.to_str().unwrap_or(""));

        let core = self.core.clone(); // you'll need Arc<Mutex<FsCore>>
        tokio::spawn(async move {
            core.with_inner(|inner| {
                if let Some(&ino) = inner.path_to_ino.get(&path) {
                    if let Some(attr) = inner.inode_attrs.get(&ino) {
                        reply.entry(&TTL, attr, 0);
                    } else {
                        reply.error(ENOENT);
                    }
                } else {
                    reply.error(ENOENT);
                }
            })
            .await;
        });
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        let core = self.core.clone();
        tokio::task::block_in_place(|| {
            let inner = core.blocking_lock_inner();
            if let Some(attr) = inner.inode_attrs.get(&ino) {
                reply.attr(&TTL, attr);
            } else {
                tracing::info!(
                    "getattr inode: {}",
                    ino,
                    // parent
                );
                reply.error(ENOENT);
            }
        });
    }

    // fn mkdir(
    //         &mut self,
    //         _req: &Request<'_>,
    //         parent: u64,
    //         name: &OsStr,
    //         mode: u32,
    //         umask: u32,
    //         reply: ReplyEntry,
    //     ) {

    // }

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
        let core = self.core.clone(); // you'll need Arc<Mutex<FsCore>>
        tokio::spawn(async move {
            core.with_inner(|inner| {
                if let Some(data) = inner.inode_data.get(&ino) {
                    let start = offset as usize;
                    let end = (start + size as usize).min(data.len());
                    if start >= data.len() {
                        reply.data(&[]);
                    } else {
                        reply.data(&data[start..end]);
                    }
                } else {
                    reply.error(ENOENT);
                }
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
        if ino != crate::ROOT_INO {
            reply.error(ENOENT);
            return;
        }

        let core = self.core.clone(); // Arc<FsCore>

        // Use tokio::task::block_in_place because readdir must be synchronous
        tokio::task::block_in_place(|| {
            let entries = {
                let inner = core.blocking_lock_inner();
                let mut entries = vec![
                    (ino, FileType::Directory, ".".into()),
                    (crate::ROOT_INO, FileType::Directory, "..".into()),
                ];

                if let Some(children) = inner.parent_to_children.get(&ino) {
                    for (name, &child_ino) in children {
                        if let Some(attr) = inner.inode_attrs.get(&child_ino) {
                            entries.push((child_ino, attr.kind, name.clone()));
                        }
                    }
                }

                entries
            };

            for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
                if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                    break;
                }
            }

            reply.ok();
        });
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        let core = self.core.clone(); // you'll need Arc<Mutex<FsCore>>
        tokio::spawn(async move {
            core.with_inner(|inner| {
                if inner.inode_attrs.contains_key(&ino) {
                    // Return 0 ,we don’t track handles yet, will return a file handle (fh)
                    reply.opened(0, 0);
                } else {
                    reply.error(libc::ENOENT);
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
        let core = self.core.clone(); // you'll need Arc<Mutex<FsCore>>
        let data = data.to_vec(); // <-- clone the slice into an owned Vec

        tokio::spawn(async move {
            core.with_inner(|inner| {
                let written = inner.write_and_persist_inode(ino, offset as u64, &data);
                reply.written(written as u32);
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
        flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        // let path = format!("/{}", name.to_string_lossy());

        tracing::trace!(
            "FUSE create request for file '{}' in parent inode {}",
            name.to_str().unwrap(),
            parent
        );
        let core = self.core.clone(); // you'll need Arc<Mutex<FsCore>>
        let name = name.to_owned();

        tokio::spawn(async move {
            match core.create_file(parent, name.to_str().unwrap(), b"").await {
                Ok(ino) => {
                    core.with_inner(|inner| {
                        if let Some(attr) = inner.inode_attrs.get(&ino) {
                            reply.created(
                                &Duration::from_secs(1),
                                attr,
                                0, // generation
                                flags.try_into().unwrap(),
                                0, // open_flags
                            );
                        } else {
                            tracing::error!("Missing inode attr after creation");
                            reply.error(ENOENT);
                        }
                    })
                    .await;
                }
                Err(e) => {
                    tracing::error!("create_file failed: {:?}", e);
                    reply.error(EIO);
                }
            }
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
                if let Some(attr) = inner.inode_attrs.get_mut(&ino) {
                    if let Some(new_size) = size {
                        let data = inner.inode_data.entry(ino).or_insert_with(Vec::new);
                        data.resize(new_size as usize, 0);
                        attr.size = new_size;
                    }

                    if let Some(new_uid) = uid {
                        attr.uid = new_uid;
                    }
                    if let Some(new_gid) = gid {
                        attr.gid = new_gid;
                    }
                    if let Some(new_mtime) = mtime {
                        attr.mtime = match new_mtime {
                            TimeOrNow::SpecificTime(t) => t,
                            TimeOrNow::Now => SystemTime::now(),
                        };
                    }
                    if let Some(new_atime) = atime {
                        attr.atime = match new_atime {
                            TimeOrNow::SpecificTime(t) => t,
                            TimeOrNow::Now => SystemTime::now(),
                        };
                    }
                    if let Some(new_ctime) = ctime {
                        attr.ctime = new_ctime;
                    }

                    reply.attr(&TTL, attr);
                } else {
                    reply.error(libc::ENOENT);
                }
            })
            .await;
        });
    }

    // Implement more methods: readdir, read, write, etc.
}
