use clap::Parser;
use std::process;

#[tokio::main]
async fn main() {
    tracing_log::LogTracer::init().expect("Failed to set up LogTracer");

    let _ = tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
    let cli = fs_core::Cli::parse();

    if let Err(e) = match &cli.command {
        fs_core::Commands::Format { device } => {
            tracing::info!("Running format on {}", device.display());
            fs_core::fs::format(device)
        }
        fs_core::Commands::Mount { device, mountpoint } => {
            tracing::info!("Mounting {} to {}", device.display(), mountpoint.display());
            fs_core::fs::mount(device, mountpoint).await
        }
        fs_core::Commands::Debug { device } => {
            tracing::info!("Debug info {}", device.display());
            fs_core::fs::debug(device)
        }
        fs_core::Commands::Serve { device, mountpoint } => {
            tracing::info!(
                "Serving filesystem on {} mounted at {}",
                device.display(),
                mountpoint.display()
            );
            fs_core::fs::mount(device, mountpoint).await
        }
    } {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
