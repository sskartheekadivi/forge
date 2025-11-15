use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use anyhow::{Result, anyhow};
use indicatif::{ProgressBar, ProgressStyle};
use nix::ioctl_read;

const BUFFER_SIZE: usize = 1024 * 1024; // 1 MiB

// Define ioctl_read for BLKGETSIZE64 (returns u64 device size in bytes)
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
        // .custom_flags(libc::O_DIRECT) // O_DIRECT is optional
        .open(&device_path)?;

    // Use nix ioctl wrapper to get device size
    let fd = device_file.as_raw_fd();
    let mut size_bytes: u64 = 0;
    unsafe {
        blkgetsize64(fd, &mut size_bytes)?;
    }

    if size_bytes == 0 {
        return Err(anyhow!("Device size is reported as zero"));
    }

    let mut image_file = File::create(&image_path)?;

    let read_pb = make_progress_bar(size_bytes, "Reading");
    let start_time = Instant::now();

    // Align buffer to 512 bytes for O_DIRECT
    let block_size = 512;
    let mut buf = vec![0u8; BUFFER_SIZE + block_size];
    let offset = buf.as_ptr().align_offset(block_size);
    let buffer = &mut buf[offset..offset + BUFFER_SIZE];

    let mut read_total: u64 = 0;
    while read_total < size_bytes {
        if !running.load(Ordering::SeqCst) {
            read_pb.println("Received exit signal... cleaning up.");
            read_pb.finish_with_message("Read cancelled.");
            // We need to clean up the partially written image file
            std::fs::remove_file(image_path)?;
            return Err(anyhow!("Operation cancelled by user"));
        }

        let to_read = std::cmp::min(BUFFER_SIZE as u64, size_bytes - read_total) as usize;

        device_file.read_exact(&mut buffer[..to_read])?;

        // This code ensures the buffer is block-aligned,
        // then writes the (potentially padded) buffer to the image file.
        let padded_size = if to_read % block_size != 0 {
            let pad = to_read.div_ceil(block_size) * block_size;
            buffer[to_read..pad].fill(0);
            pad
        } else {
            to_read
        };

        image_file.write_all(&buffer[..padded_size])?;
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
