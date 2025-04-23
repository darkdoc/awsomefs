use std::fs::File;
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;

pub struct BlockDevice {
    pub file: File,
}

impl BlockDevice {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = File::options().read(true).write(true).open(path)?;
        Ok(Self { file })
    }

    pub fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read(buf)
    }

    pub fn write_at(&mut self, offset: u64, buf: &[u8]) -> std::io::Result<usize> {
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write(buf)
    }
}
