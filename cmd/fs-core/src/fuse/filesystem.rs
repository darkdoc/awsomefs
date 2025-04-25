use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen,
    Request, TimeOrNow,
};

use libc::{EIO, ENOENT};
use std::ffi::OsStr;
use std::sync::{Arc,Mutex};
use std::time::{Duration, SystemTime};
// , UNIX_EPOCH};

const TTL: Duration = Duration::from_secs(1); // 1 second
const ROOT_INO: u64 = 1;

pub struct AwsomeFs {
    core: Arc<Mutex<crate::FsCore>>,
}

impl AwsomeFs {
    pub fn new(core: Arc<Mutex<crate::FsCore>>) -> Self {
        Self { core }
    }
}

impl Filesystem for AwsomeFs {
    fn lookup(&mut self, _req: &Request<'_>, _parent: u64, name: &OsStr, reply: ReplyEntry) {
        // Stub
        let path = format!("/{}", name.to_str().unwrap_or(""));
        if let Some(&ino) = self.core.path_to_ino.get(&path) {
            if let Some(attr) = self.core.inode_attrs.get(&ino) {
                reply.entry(&TTL, attr, 0);
            } else {
                reply.error(ENOENT);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        if let Some(attr) = self.core.inode_attrs.get(&ino) {
            reply.attr(&TTL, attr);
        } else {
            reply.error(ENOENT);
        }
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
        if let Some(data) = self.core.inode_data.get(&ino) {
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
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino != ROOT_INO {
            reply.error(ENOENT);
            return;
        }

        let mut entries = vec![
            (ROOT_INO, FileType::Directory, ".".into()),
            (ROOT_INO, FileType::Directory, "..".into()),
        ];

        for (path, &ino) in &self.core.path_to_ino {
            if path != "/" {
                if let Some(attr) = self.core.inode_attrs.get(&ino) {
                    let name: String = path.trim_start_matches('/').into();
                    entries.push((ino, attr.kind, name));
                }
            }
        }

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        if self.core.inode_attrs.contains_key(&ino) {
            // Return 0 ,we don’t track handles yet, will return a file handle (fh)
            reply.opened(0, 0);
        } else {
            reply.error(libc::ENOENT);
        }
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
        let written = self.core.write_and_persist_inode(ino, offset as u64, data);
        reply.written(written as u32);
        // return some error if write failed
        // reply.error(EIO);
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
        let path = format!("/{}", name.to_string_lossy());

        tracing::trace!(
            "FUSE create request for file '{}' in parent inode {}",
            path,
            parent
        );
        let core = self.core.clone(); // you'll need Arc<Mutex<FsCore>>


        tokio::spawn(async move {
            let mut core = match core.lock() {
                Ok(c) => c,
                Err(poisoned) => poisoned.into_inner(),
            };

            match core.create_file(&path, b"").await {
                Ok(ino) => {
                    if let Some(attr) = core.inode_attrs.get(&ino) {
                        reply.created(
                            &Duration::from_secs(1), // entry_valid
                            attr,
                            0, // generation
                            flags.try_into().unwrap(),
                            0, // open_flags
                        );
                    } else {
                        tracing::error!("Missing inode attr after creation");
                        reply.error(ENOENT);
                    }
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
        if let Some(attr) = self.core.inode_attrs.get_mut(&ino) {
            if let Some(new_size) = size {
                let data = self.core.inode_data.entry(ino).or_insert_with(Vec::new);
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
    }

    // Implement more methods: readdir, read, write, etc.
}
