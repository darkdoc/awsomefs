// use std::fs::{File};
use std::io::Result;
// use std::io::{Result, SeekFrom};

// use std::os::unix::fs::FileExt;
// use std::os::unix::fs::OpenOptionsExt;
use fuser::{mount2, MountOption};
use std::path::Path;
use tokio::time::{timeout, Duration};

use crate::local::LocalMetadataCoordinator;
use crate::metadata::MetadataCoordinator;
use crate::remote::RemoteMetadataCoordinator;
use crate::AwsomeFs;
use crate::BlockDevice;
use crate::FsCore;
use crate::Superblock;

pub fn format<P: AsRef<Path>>(device_path: P) -> Result<()> {
    tracing::info!("Formatting device: {:?}", device_path.as_ref());

    let mut bd = BlockDevice::open(&device_path, 4096)?;
    // // Write a magic header or initialize metadata block

    let sb = Superblock::new(4096, 1);
    sb.save(&mut bd.file,bd.block_size).unwrap();

    // Here you would write superblock, reserve journal, etc.
    tracing::info!("Format complete.");
    Ok(())
}

pub async fn mount<P: AsRef<Path>>(device_path: P, mountpoint: P) -> Result<()> {
    let mut bd = BlockDevice::open(&device_path, 4096)?;

    let _loaded = Superblock::load(&mut bd.file, bd.block_size).unwrap();

    let options = vec![
        MountOption::RW,
        MountOption::FSName("AwesomeFS".to_string()),
        MountOption::AutoUnmount,
        MountOption::AllowRoot,
    ];

    // Try remote coordinator with timeout
    let coordinator = match timeout(
        Duration::from_secs(1),
        RemoteMetadataCoordinator::connect("http://127.0.0.1:50051"),
    )
    .await
    {
        Ok(Ok(remote)) => {
            tracing::info!("Connected to remote metadata coordinator");
            Box::new(remote) as Box<dyn MetadataCoordinator>
        }
        Ok(Err(e)) => {
            tracing::warn!(
                "Failed to connect to remote coordinator: {}. Falling back to local.",
                e
            );
            Box::new(LocalMetadataCoordinator::new()) as Box<dyn MetadataCoordinator>
        }
        Err(_) => {
            tracing::warn!(
                "Timeout while connecting to remote coordinator. Falling back to local."
            );
            Box::new(LocalMetadataCoordinator::new()) as Box<dyn MetadataCoordinator>
        }
    };

    let fs_core = FsCore::with_coordinator(bd, coordinator);

    let fs = AwsomeFs::new(fs_core).await?;

    mount2(fs, &mountpoint, &options)?;

    tracing::info!("Mount successful");
    Ok(())
}

pub fn debug<P: AsRef<Path>>(device_path: P) -> Result<()> {
    tracing::info!("Device info: {:?}", device_path.as_ref());

    let mut bd = BlockDevice::open(&device_path, 4096)?;

    let loaded = Superblock::load(&mut bd.file, bd.block_size).unwrap();

    tracing::info!("Superblock {:?}", loaded);
    Ok(())
}
