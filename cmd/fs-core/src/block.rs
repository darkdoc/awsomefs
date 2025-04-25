use std::fs::File;
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;

pub struct BlockDevice {
    pub file: File,
    pub block_size: usize,

}

impl BlockDevice {
    pub fn open<P: AsRef<Path>>(path: P,block_size:usize ) -> std::io::Result<Self> {
        let file = File::options().read(true).write(true).open(path)?;
        Ok(Self { file,block_size })
    }

    pub fn read_block(&mut self, block_num: u64, buf: &mut [u8]) -> std::io::Result<()> {
        self.file.seek(SeekFrom::Start(block_num * self.block_size as u64))?;
        self.file.read_exact(buf)?;
        Ok(())

    }

    pub fn write_block(&mut self, block_num: u64, buf: &[u8]) -> std::io::Result<()> {
        self.file.seek(SeekFrom::Start(block_num * self.block_size as u64))?;
        self.file.write_all(buf)?;
        Ok(())
    }
}
