use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};

mod write;
mod read;

#[derive(Parser)]
#[command(name = "forge")]
#[command(about = "Disk imaging tool", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Write {
        /// Image file to write (supports only uncompressed images for now)
        #[arg()]
        image: Option<PathBuf>,

        /// Target device to write to (e.g., /dev/sdX)
        #[arg()]
        device: Option<PathBuf>,

        /// Skip write verification
        #[arg(short = 'n', long = "no-verify")]
        no_verify: bool,
    },
    Read {
        /// Target device to read from (e.g., /dev/sdX)
        #[arg()]
        device: Option<PathBuf>,

        /// Output image file
        #[arg()]
        image: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Write { image, device, no_verify, .. } => {
            let image = image.ok_or_else(|| anyhow!("Missing image argument"))?;
            let device = device.ok_or_else(|| anyhow!("Missing device argument"))?;
            write::run(&image, Path::new(&device), !no_verify)?;
        }
        Commands::Read { device, image } => {
            read::run(device, image)?;
        }
    }

    Ok(())
}

