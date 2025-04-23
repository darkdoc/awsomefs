use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen,
    Request, TimeOrNow,
};

use libc::{EIO, ENOENT};
use std::ffi::OsStr;
use std::time::{Duration, SystemTime};
// , UNIX_EPOCH};

use std::collections::HashMap;

const TTL: Duration = Duration::from_secs(1); // 1 second
const ROOT_INO: u64 = 1;

pub struct FsCore {
    inode_counter: u64,
    inode_attrs: HashMap<u64, FileAttr>,
    inode_data: HashMap<u64, Vec<u8>>,
    path_to_ino: HashMap<String, u64>,
}

impl FsCore {
    pub fn new() -> Self {
        let mut fs = FsCore {
            inode_counter: 2, // 1 = root, start from 2
            inode_attrs: HashMap::new(),
            inode_data: HashMap::new(),
            path_to_ino: HashMap::new(),
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
        let ino = fs.allocate_inode();
        let content = b"hello world\n".to_vec();
        let attr = FileAttr {
            ino,
            size: content.len() as u64,
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

        fs.inode_attrs.insert(ino, attr);
        fs.inode_data.insert(ino, content);
        fs.path_to_ino.insert("/hello.txt".into(), ino);

        fs
    }

    fn allocate_inode(&mut self) -> u64 {
        let ino = self.inode_counter;
        self.inode_counter += 1;
        ino
    }

    pub fn write_to_inode(&mut self, ino: u64, offset: u64, data: &[u8]) -> usize {
        // Get or create the data buffer for this inode
        let file_data = self.inode_data.entry(ino).or_insert_with(Vec::new);

        let end_offset = (offset as usize) + data.len();
        if file_data.len() < end_offset {
            file_data.resize(end_offset, 0);
        }

        file_data[offset as usize..end_offset].copy_from_slice(data);

        // Update file size in FileAttr
        if let Some(attr) = self.inode_attrs.get_mut(&ino) {
            attr.size = file_data.len() as u64;
        }

        data.len()
    }
}

pub struct AwsomeFs {
    core: FsCore,
}

impl AwsomeFs {
    pub fn new(core: FsCore) -> Self {
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
            // (2, FileType::RegularFile, "hello.txt"),
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
        let written = self.core.write_to_inode(ino, offset as u64, data);
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
        _flags: i32,
        reply: fuser::ReplyCreate,
    ) {
        log::trace!("create(parent: {}, name: {:?})", parent, name);

        let name_str = name.to_str().unwrap_or("");
        let full_path = format!("/{name_str}");

        let ino = self.core.inode_counter + 1;
        self.core.inode_counter = ino;

        let attr = FileAttr {
            ino,
            size: 0,
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

        self.core.inode_attrs.insert(ino, attr);
        self.core.path_to_ino.insert(full_path.clone(), ino);
        self.core.inode_data.insert(ino, vec![]);

        reply.created(&TTL, &attr, 0, 0, 0);
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
                };            }

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
