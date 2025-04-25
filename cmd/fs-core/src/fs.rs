// use std::fs::{File};
use std::io::Result;
// use std::io::{Result, SeekFrom};
use std::sync::{Arc, Mutex};

// use std::os::unix::fs::FileExt;
// use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use fuser::{mount2,MountOption};
use tokio::time::{timeout, Duration};


use crate::remote::RemoteMetadataCoordinator;
use crate::local::LocalMetadataCoordinator;
use crate::metadata::MetadataCoordinator;
use crate::BlockDevice;
use crate::FsCore;
use crate::Superblock;
use crate::AwsomeFs;

pub fn format<P: AsRef<Path>>(device_path: P) -> Result<()> {
    tracing::info!("Formatting device: {:?}", device_path.as_ref());

    let mut bd = BlockDevice::open(&device_path,4096)?;
    // // Write a magic header or initialize metadata block
    // file.write_all(b"AWESOMEFS")?;

    let sb = Superblock::new(4096, 100);
    sb.save(&mut bd.file).unwrap();

    // Here you would write superblock, reserve journal, etc.
    tracing::info!("Format complete.");
    Ok(())
}

pub async fn mount<P: AsRef<Path>>(device_path: P, mountpoint: P) ->Result<Arc<Mutex<FsCore>>> {
    let mut bd = BlockDevice::open(&device_path,4096)?;

    let _loaded = Superblock::load(&mut bd.file).unwrap();

    let options = vec![
        MountOption::RW, 
        MountOption::FSName("AwesomeFS".to_string()),
        MountOption::AutoUnmount,
        MountOption::AllowRoot,
        ];
    
    // Try remote coordinator with timeout
    let coordinator = match timeout(
        Duration::from_secs(1),
        RemoteMetadataCoordinator::connect("http://127.0.0.1:50051")
    ).await {
        Ok(Ok(remote)) => {
            tracing::info!("Connected to remote metadata coordinator");
            Box::new(remote) as Box<dyn MetadataCoordinator>
        }
        Ok(Err(e)) => {
            tracing::warn!("Failed to connect to remote coordinator: {}. Falling back to local.", e);
            Box::new(LocalMetadataCoordinator::new()) as Box<dyn MetadataCoordinator>
        }
        Err(_) => {
            tracing::warn!("Timeout while connecting to remote coordinator. Falling back to local.");
            Box::new(LocalMetadataCoordinator::new()) as Box<dyn MetadataCoordinator>
        }
    };
    
    let mut fs_core = Arc::new(Mutex::new(FsCore::with_coordinator(bd, coordinator)));

    // let mut fs_core = FsCore::with_coordinator(bd, coordinator);
    // Initialize from device
    {
        let mut guard = fs_core.lock().unwrap();
        guard.init()?; // Now inside Arc<Mutex<_>>
    }
    let fs = AwsomeFs::new(fs_core.clone());

    // tracing::dispatcher::get_default(|d| {
    //     d.downcast_ref::<tracing_subscriber::FmtSubscriber>()
    //         .map(|s| s.flush())
    // });

    mount2(fs, &mountpoint, &options)?;


    tracing::info!("Mount successful");
    Ok(fs_core)
}

pub fn debug<P: AsRef<Path>>(device_path: P) -> Result<()> {
    tracing::info!("Device info: {:?}", device_path.as_ref());

    let mut bd = BlockDevice::open(&device_path,4096)?;

    let loaded = Superblock::load(&mut bd.file).unwrap();
    
    tracing::info!("Superblock {:?}", loaded);
    // loaded
    // file.seek(SeekFrom::Start((0)))?; //read_at(buf, offset)
    // // Write a magic header or initialize metadata block
    // file.write_all(b"AWESOMEFS")?;

    // Here you would write superblock, reserve journal, etc.
    Ok(())
}