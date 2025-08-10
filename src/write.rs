use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::time::Instant;

use anyhow::{Result, anyhow};
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};

const BUFFER_SIZE: usize = 1024 * 1024; // 1 MiB

fn make_progress_bar(len: u64, prefix: &str, color: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_prefix(format!("{prefix:<10}"));
    pb.set_style(
        ProgressStyle::default_bar()
            .template(&format!("{{prefix}} [{{elapsed_precise}}] [{{bar:40.{color}/black}}] {{bytes}}/{{total_bytes}} ({{bytes_per_sec}}, {{eta}}) {{msg}}"))
            .unwrap()
            .progress_chars("■ "),
    );
    pb
}

pub fn run(image_path: &Path, device_path: &Path, verify: bool) -> Result<()> {
    println!(
        "Writing image \"{}\" to device \"{}\"",
        image_path.display(),
        device_path.display()
    );

    // --- Writing ---
    let mut image_file = File::open(image_path)?;
    let image_len = image_file.metadata()?.len();

    let mut device_file = std::fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_DIRECT)
        .open(device_path)?;

    let write_pb = make_progress_bar(image_len, "Writing", "green");
    let start_time = Instant::now();

    // Align buffer to 512 bytes for O_DIRECT
    let block_size = 512;
    let mut buf = vec![0u8; BUFFER_SIZE + block_size];
    let offset = buf.as_ptr().align_offset(block_size);
    let buffer = &mut buf[offset..offset + BUFFER_SIZE];

    let mut written: u64 = 0;
    while written < image_len {
        let to_read = std::cmp::min(BUFFER_SIZE as u64, image_len - written) as usize;
        image_file.read_exact(&mut buffer[..to_read])?;

        let padded_size = if to_read % block_size != 0 {
            let pad = to_read.div_ceil(block_size) * block_size;
            buffer[to_read..pad].fill(0);
            pad
        } else {
            to_read
        };

        device_file.write_all(&buffer[..padded_size])?;
        written += to_read as u64;
        write_pb.set_position(written);
    }

    device_file.flush()?;

    let write_elapsed = start_time.elapsed().as_secs_f64();
    let write_avg_speed = (image_len as f64 / (1024.0 * 1024.0)) / write_elapsed;
    write_pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{prefix} [{elapsed_precise}] [{bar:40.green/black}] {total_bytes} (avg {msg}",
            )
            .unwrap()
            .progress_chars("■ "),
    );
    write_pb.finish_with_message(format!(
        "{write_avg_speed:6.2} MiB/s, {write_elapsed:5.1}s) ✅ Write complete."
    ));

    println!();

    // --- Verification ---
    if verify {
        let mut image_file = File::open(image_path)?;
        let mut device_file = File::open(device_path)?;

        let verify_pb = make_progress_bar(image_len, "Verifying", "magenta");
        let verify_start = Instant::now();

        let mut image_hasher = Sha256::new();
        let mut device_hasher = Sha256::new();

        let mut image_buf = vec![0u8; BUFFER_SIZE];
        let mut device_buf = vec![0u8; BUFFER_SIZE];

        let mut remaining = image_len;
        while remaining > 0 {
            let chunk = std::cmp::min(BUFFER_SIZE as u64, remaining) as usize;
            image_file.read_exact(&mut image_buf[..chunk])?;
            device_file.read_exact(&mut device_buf[..chunk])?;

            image_hasher.update(&image_buf[..chunk]);
            device_hasher.update(&device_buf[..chunk]);

            verify_pb.inc(chunk as u64);
            remaining -= chunk as u64;
        }

        let verify_elapsed = verify_start.elapsed().as_secs_f64();
        let verify_avg_speed = (image_len as f64 / (1024.0 * 1024.0)) / verify_elapsed;

        let hash1 = image_hasher.finalize();
        let hash2 = device_hasher.finalize();

        verify_pb.set_style(
            ProgressStyle::default_bar()
                .template("{prefix} [{elapsed_precise}] [{bar:40.magenta/black}] {total_bytes} (avg {msg}")
                .unwrap()
                .progress_chars("■ "),
        );

        if hash1 == hash2 {
            verify_pb.finish_with_message(format!(
                "{verify_avg_speed:6.2} MiB/s, {verify_elapsed:5.1}s) ✅ Verification successful."
            ));
        } else {
            return Err(anyhow!(
                "❌ Verification failed: hash mismatch. (avg {:.2} MiB/s)",
                verify_avg_speed
            ));
        }
    }

    Ok(())
}
