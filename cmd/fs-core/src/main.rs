use clap::Parser;
use std::process;
use log::info;

fn main() {
    env_logger::init();
    let cli = fs_core::Cli::parse();

    if let Err(e) = match &cli.command {
        fs_core::Commands::Format { device } => {
            info!("Running format on {}", device.display());
            fs_core::fs::format(device)
        }
        fs_core::Commands::Mount { device, mountpoint } => {
            info!("Mounting {} to {}", device.display(), mountpoint.display());
            fs_core::fs::mount(device, mountpoint)
        }
        fs_core::Commands::Debug { device } => {
            info!("Debug info {}", device.display());
            fs_core::fs::debug(device)
        }
        fs_core::Commands::Serve { device, mountpoint } => {
            info!("Serving filesystem on {} mounted at {}", device.display(), mountpoint.display());
            fs_core::fs::mount(device, mountpoint) // placeholder — later this could run a FUSE server
        }
    } {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
