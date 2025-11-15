use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::time::Instant;

// Required for .custom_flags(libc::O_DIRECT)
use std::os::unix::fs::OpenOptionsExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Result, anyhow};
use indicatif::{ProgressBar, ProgressStyle};
use nix::ioctl_read;

// Use a 1 MiB buffer for I/O operations.
const BUFFER_SIZE: usize = 1024 * 1024;

// Define the `nix` ioctl for `BLKGETSIZE64` (u64 device size in bytes).
ioctl_read!(blkgetsize64, 0x12, 114, u64);

fn make_progress_bar(len: u64, prefix: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_prefix(format!("{prefix:<10}"));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{prefix} [{elapsed_precise}] [{bar:40.green/black}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {msg}")
            .unwrap()
            .progress_chars("■ "),
    );
    pb
}

pub fn run(device_path: &Path, image_path: &Path, running: Arc<AtomicBool>) -> Result<()> {
    println!(
        "Reading device \"{}\" to image \"{}\"",
        device_path.display(),
        image_path.display()
    );

    // Open device for reading
    let mut device_file = std::fs::OpenOptions::new()
        .read(true)
        // Use O_DIRECT to bypass the kernel page cache for raw, high-speed I/O.
        .custom_flags(libc::O_DIRECT)
        .open(&device_path)?;

    // Get the device size in bytes using ioctl. This is more reliable
    // than seeking for block devices.
    let fd = device_file.as_raw_fd();
    let mut size_bytes: u64 = 0;
    unsafe {
        blkgetsize64(fd, &mut size_bytes)?;
    }

    // Abort if the device reports zero size (e.g., empty card reader).
    if size_bytes == 0 {
        return Err(anyhow!("Device size is reported as zero"));
    }

    let mut image_file = File::create(&image_path)?;

    let read_pb = make_progress_bar(size_bytes, "Reading");
    let start_time = Instant::now();

    // O_DIRECT requires buffers to be memory-aligned to the block size.
    // We create a buffer with extra capacity and then get an aligned slice from it.
    let block_size = 512;
    let mut buf = vec![0u8; BUFFER_SIZE + block_size];
    let offset = buf.as_ptr().align_offset(block_size);
    let buffer = &mut buf[offset..offset + BUFFER_SIZE];

    let mut read_total: u64 = 0;
    while read_total < size_bytes {
        // Check for Ctrl+C signal for graceful shutdown.
        if !running.load(Ordering::SeqCst) {
            read_pb.println("Received exit signal... cleaning up.");
            read_pb.finish_with_message("❌ Read cancelled.");
            // Clean up the partial image file on cancellation.
            std::fs::remove_file(image_path)?;
            return Err(anyhow!("Operation cancelled by user"));
        }

        let to_read = std::cmp::min(BUFFER_SIZE as u64, size_bytes - read_total) as usize;

        device_file.read_exact(&mut buffer[..to_read])?;

        // Write *only* the bytes read. Do not write the full buffer,
        // as the last chunk will be partial and uninitialized data
        // from the buffer would corrupt the image.
        image_file.write_all(&buffer[..to_read])?;

        read_total += to_read as u64;
        read_pb.set_position(read_total);
    }

    image_file.flush()?;

    let elapsed = start_time.elapsed().as_secs_f64();
    let avg_speed = (size_bytes as f64 / (1024.0 * 1024.0)) / elapsed;
    read_pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{prefix} [{elapsed_precise}] [{bar:40.green/black}] {total_bytes} (avg {msg}",
            )
            .unwrap()
            .progress_chars("■ "),
    );
    read_pb.finish_with_message(format!(
        "{avg_speed:.2} MiB/s, {elapsed:.1}s) ✅ Read complete."
    ));

    let metadata = image_file.metadata()?;
    let actual_size = metadata.len();
    println!(
        "Read complete: \"{}\" ({} bytes, {:.2} MiB)",
        image_path.display(),
        actual_size,
        actual_size as f64 / (1024.0 * 1024.0)
    );

    Ok(())
}
