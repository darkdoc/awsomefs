use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "awesomefs")]
#[command(about = "CLI to interact with the awesomefs filesystem", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Format the raw device with AwesomeFS
    Format {
        #[arg(short, long)]
        device: PathBuf,
    },
    /// Mount the filesystem
    Mount {
        #[arg(short, long)]
        device: PathBuf,
        #[arg(short, long)]
        mountpoint: PathBuf,
    },
    /// Start the filesystem service (future: with FUSE)
    Serve {
        #[arg(value_name = "DEVICE")]
        device: PathBuf,

        #[arg(value_name = "MOUNTPOINT")]
        mountpoint: PathBuf,
    },
    /// Print debug info about a filesystem
    Debug {
        #[arg(short, long)]
        device: PathBuf,
    },
}