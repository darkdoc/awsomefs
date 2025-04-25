use serde::{Serialize, Deserialize};
use std::time::SystemTime;
use fuser::{FileAttr, FileType};

#[derive(Serialize, Deserialize)]
pub enum SerializableFileType {
    RegularFile,
    Directory,
    Symlink,
    CharDevice,
    BlockDevice,
    NamedPipe,
    Socket,
}

impl From<FileType> for SerializableFileType {
    fn from(ft: FileType) -> Self {
        match ft {
            FileType::RegularFile => Self::RegularFile,
            FileType::Directory => Self::Directory,
            FileType::Symlink => Self::Symlink,
            FileType::CharDevice => Self::CharDevice,
            FileType::BlockDevice => Self::BlockDevice,
            FileType::NamedPipe => Self::NamedPipe,
            FileType::Socket => Self::Socket,
        }
    }
}

impl From<SerializableFileType> for FileType {
    fn from(sft: SerializableFileType) -> Self {
        match sft {
            SerializableFileType::RegularFile => FileType::RegularFile,
            SerializableFileType::Directory => FileType::Directory,
            SerializableFileType::Symlink => FileType::Symlink,
            SerializableFileType::CharDevice => FileType::CharDevice,
            SerializableFileType::BlockDevice => FileType::BlockDevice,
            SerializableFileType::NamedPipe => FileType::NamedPipe,
            SerializableFileType::Socket => FileType::Socket,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializableFileAttr {
    pub ino: u64,
    pub size: u64,
    pub blocks: u64,
    pub atime: SystemTime,
    pub mtime: SystemTime,
    pub ctime: SystemTime,
    pub crtime: SystemTime,
    pub kind: SerializableFileType,
    pub perm: u16,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u32,
    pub flags: u32,
    pub blksize: u32,
}

impl From<FileAttr> for SerializableFileAttr {
    fn from(attr: FileAttr) -> Self {
        Self {
            ino: attr.ino,
            size: attr.size,
            blocks: attr.blocks,
            atime: attr.atime,
            mtime: attr.mtime,
            ctime: attr.ctime,
            crtime: attr.crtime,
            kind: attr.kind.into(),
            perm: attr.perm,
            nlink: attr.nlink,
            uid: attr.uid,
            gid: attr.gid,
            rdev: attr.rdev,
            flags: attr.flags,
            blksize: attr.blksize,
        }
    }
}

impl From<SerializableFileAttr> for FileAttr {
    fn from(attr: SerializableFileAttr) -> Self {
        Self {
            ino: attr.ino,
            size: attr.size,
            blocks: attr.blocks,
            atime: attr.atime,
            mtime: attr.mtime,
            ctime: attr.ctime,
            crtime: attr.crtime,
            kind: attr.kind.into(),
            perm: attr.perm,
            nlink: attr.nlink,
            uid: attr.uid,
            gid: attr.gid,
            rdev: attr.rdev,
            flags: attr.flags,
            blksize: attr.blksize,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PersistedInode {
    pub attr: SerializableFileAttr,
    pub data: Vec<u8>,
    pub path: String,
}