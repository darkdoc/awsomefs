use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

const SUPERBLOCK_OFFSET: u64 = 0;
const SUPERBLOCK_MAGIC: u64 = 0xAABBCCDD11223344;
const SUPERBLOCK_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct Superblock {
    pub magic: u64,       // Magic number for identification
    pub version: u32,     // Filesystem version
    pub uuid: [u8; 16],   // basic uuid field
    pub block_size: u32,  // Block size in bytes
    pub inode_count: u64, // Total number of inodes
    // pub block_count: u64,      // Total number of blocks
    // pub free_block_count: u64, // Free block count
    pub free_inode_count: u64, // Free inode count
}

impl Superblock {
    pub fn new(block_size: u32, total_inodes: u64) -> Self {
        let uuid = uuid::Uuid::new_v4().as_bytes().clone();
        Self {
            magic: SUPERBLOCK_MAGIC,
            version: SUPERBLOCK_VERSION,
            uuid,
            block_size,
            inode_count: total_inodes,
            free_inode_count: total_inodes,
        }
    }

    pub fn load(file: &mut File) -> std::io::Result<Self> {
        file.seek(SeekFrom::Start(SUPERBLOCK_OFFSET))?;
        let mut buf = [0u8; 512];
        file.read_exact(&mut buf)?;
        let sb: Superblock = bincode::deserialize(&buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        if sb.magic != SUPERBLOCK_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid magic number",
            ));
        }
        Ok(sb)
    }

    pub fn save(&self, file: &mut File) -> std::io::Result<()> {
        file.seek(SeekFrom::Start(SUPERBLOCK_OFFSET))?;
        let buf = bincode::serialize(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let mut padded = vec![0u8; 512];
        padded[..buf.len()].copy_from_slice(&buf);
        file.write_all(&padded)?;
        Ok(())
    }
}
