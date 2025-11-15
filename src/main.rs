use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use libc::ECHOCTL;
use std::io::{IsTerminal, stdout};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use termios::{TCSANOW, Termios, tcsetattr};

mod device;
mod read;
mod write;

#[derive(Parser)]
#[command(name = "forge")]
#[command(about = "A safe, interactive disk imaging tool", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Write an image to a device interactively
    Write {
        /// Image file to write
        #[arg(required = true)]
        image: PathBuf,

        /// Skip write verification
        #[arg(short = 'n', long = "no-verify")]
        no_verify: bool,
    },
    /// Read a device to an image file interactively
    Read {
        /// Output image file
        #[arg(required = true)]
        image: PathBuf,
    },
    /// List available removable devices
    List,
}

struct TermRestorer {
    original_termios: Option<Termios>,
}

impl TermRestorer {
    fn new() -> Self {
        let fd = stdout().as_raw_fd();
        if !stdout().is_terminal() {
            return Self {
                original_termios: None,
            };
        }

        if let Ok(original_termios) = Termios::from_fd(fd) {
            let mut new_termios = original_termios;
            new_termios.c_lflag &= !ECHOCTL;

            if tcsetattr(fd, TCSANOW, &new_termios).is_ok() {
                Self {
                    original_termios: Some(original_termios),
                }
            } else {
                Self {
                    original_termios: None,
                }
            }
        } else {
            Self {
                original_termios: None,
            }
        }
    }
}

impl Drop for TermRestorer {
    fn drop(&mut self) {
        if let Some(ref original_termios) = self.original_termios {
            let fd = stdout().as_raw_fd();
            tcsetattr(fd, TCSANOW, original_termios).ok();
        }
    }
}

fn main() -> Result<()> {
    // This guard will be dropped when main() exits, restoring the terminal
    let _term_restorer = TermRestorer::new();

    // Atomic boolean flag to signal termination
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Set up the Ctrl+C handler
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Write { image, no_verify } => {
            let devices = device::get_removable_devices()?;
            let device = device::select_device(&devices, "Select the target device to WRITE to")?;

            // Print the warning and operation details manually
            println!(
                "{} This will erase all data on '{}' ({:.1} GB).",
                style("WARNING:").red().bold(),
                device.name,
                device.size_gb,
            );
            println!("  Device: {}", style(device.path.display()).cyan());
            println!("  Image:  {}", style(image.display()).cyan());
            println!();

            // Create a simple prompt string for the confirmation
            let prompt = "Are you sure you want to proceed?";

            if !device::confirm_operation(&prompt, &device, &image)? {
                println!("Write operation cancelled.");
                return Ok(());
            }

            println!();
            write::run(&image, &device.path, !no_verify, running.clone())?;
            println!(
                "\n✨ Successfully flashed {} with {}.",
                style(device.path.display()).cyan(),
                style(image.display()).cyan()
            );
        }
        Commands::Read { image } => {
            let devices = device::get_removable_devices()?;
            let device = device::select_device(&devices, "Select the source device to READ from")?;

            // Print the operation details manually
            println!(
                "This will read {:.1} GB from '{}'.",
                device.size_gb, device.name
            );
            println!("  Device: {}", style(device.path.display()).cyan());
            println!("  Output: {}", style(image.display()).cyan());
            println!();

            // Create a simple prompt string for the confirmation
            let prompt = "Are you sure you want to proceed?";

            if !device::confirm_operation(&prompt, &device, &image)? {
                println!("Read operation cancelled.");
                return Ok(());
            }

            println!();
            read::run(&device.path, &image, running.clone())?;
            println!(
                "\n✨ Successfully read {} to {}.",
                style(device.path.display()).cyan(),
                style(image.display()).cyan()
            );
        }
        Commands::List => {
            let devices = device::get_removable_devices()?;
            if devices.is_empty() {
                println!("No removable devices found.");
                return Ok(());
            }

            println!("Found {} removable devices:", devices.len());
            println!(
                "\n  {:<12} {:<25} {:<10} {}",
                "DEVICE", "NAME", "SIZE", "LOCATION"
            );
            println!("  {:-<12} {:-<25} {:-<10} {:-<20}", "", "", "", "");
            for device in devices {
                let location = if device.mount_point.is_empty() {
                    "(Not mounted)".to_string()
                } else {
                    device.mount_point
                };
                println!(
                    "  {:<12} {:<25} {:>8.1} GB  {}",
                    device.path.display(),
                    device.name,
                    device.size_gb,
                    location
                );
            }
        }
    }

    Ok(())
}
