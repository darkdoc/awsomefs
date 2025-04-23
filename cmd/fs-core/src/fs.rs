// use std::fs::{File};
use std::io::Result;
// use std::io::{Result, SeekFrom};

// use std::os::unix::fs::FileExt;
// use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use fuser::{mount2,MountOption};

use crate::BlockDevice;
// use fuse::filesystem::DevFs;


pub fn format<P: AsRef<Path>>(device_path: P) -> Result<()> {
    println!("Formatting device: {:?}", device_path.as_ref());

    let mut bd = BlockDevice::open(&device_path)?;
    // // Write a magic header or initialize metadata block
    // file.write_all(b"AWESOMEFS")?;

    let sb = super::superblock::Superblock::new(4096, 100);
    sb.save(&mut bd.file).unwrap();

    // Here you would write superblock, reserve journal, etc.
    println!("Format complete.");
    Ok(())
}

pub fn mount<P: AsRef<Path>>(device_path: P, mountpoint: P) -> Result<()> {

    // This would typically call a FUSE mount handler
    // For now, stub a basic check
    let mut bd = BlockDevice::open(&device_path)?;

    let _loaded = super::superblock::Superblock::load(&mut bd.file).unwrap();

    let options = vec![
        MountOption::RW, 
        MountOption::FSName("AwesomeFS".to_string()),
        MountOption::AutoUnmount,
        MountOption::AllowRoot,
        ];
    
    let fs_core = super::FsCore::new();

    let fs = super::AwsomeFs::new(fs_core);


    mount2(fs, &mountpoint, &options).unwrap();


    println!("Mount successful (simulated).");
    Ok(())
}

pub fn debug<P: AsRef<Path>>(device_path: P) -> Result<()> {
    println!("Device info: {:?}", device_path.as_ref());

    let mut bd = BlockDevice::open(&device_path)?;

    let loaded = super::superblock::Superblock::load(&mut bd.file).unwrap();
    
    print!("Superblock {:?}", loaded);
    // loaded
    // file.seek(SeekFrom::Start((0)))?; //read_at(buf, offset)
    // // Write a magic header or initialize metadata block
    // file.write_all(b"AWESOMEFS")?;

    // Here you would write superblock, reserve journal, etc.
    Ok(())
}