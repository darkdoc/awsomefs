#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inode {
    pub inode_number: u64,
    pub size: u64,
    pub block_ptrs: [u64; 12],    // Direct blocks
    pub indirect_ptr: u64,        // Single indirect
    pub double_indirect_ptr: u64, // Double indirect
    pub triple_indirect_ptr: u64, // Triple indirect
    pub mode: u16,                // Permissions and file type
    pub uid: u32,                 // Owner
    pub gid: u32,                 // Group
    pub atime: u64,               // Last access
    pub mtime: u64,               // Last modification
    pub ctime: u64,               // Last metadata change
}