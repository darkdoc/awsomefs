pub trait BlockDevice {
    fn read_block(&mut self, block_id: u64, buf: &mut [u8]) -> std::io::Result<()>;
    fn write_block(&mut self, block_id: u64, buf: &[u8]) -> std::io::Result<()>;
}
