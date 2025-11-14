use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tempfile::{NamedTempFile, TempPath};

use anyhow::{Result, anyhow};
use console::style;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use xz2::read::XzDecoder;
use zstd::stream::read::Decoder as ZstdDecoder;

const BUFFER_SIZE: usize = 1024 * 1024; // 1 MiB

/// Manages the lifetime of a decompressed image file.
/// If the image was decompressed to a temp file, `_temp_handle` will
/// hold the `TempPath`, and the file will be deleted on drop.
/// If it was an uncompressed image, `_temp_handle` is None.
struct DecompressedImage {
    path: PathBuf,
    _temp_handle: Option<TempPath>,
}

/// Allows `DecompressedImage` to be used as a simple `&Path`.
impl AsRef<Path> for DecompressedImage {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

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

/// Decompresses an image file to a temporary file if needed.
/// Returns a `DecompressedImage` struct which points to either
/// the original file (if uncompressed) or the new temp file.
fn decompress_image(input_path: &Path) -> io::Result<DecompressedImage> {
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let input_file = File::open(input_path)?;

    // Create a reader based on the file extension
    let mut reader: Box<dyn Read> = match ext.as_str() {
        "gz" | "gzip" => Box::new(GzDecoder::new(BufReader::new(input_file))),
        "xz" => Box::new(XzDecoder::new(BufReader::new(input_file))),
        "zst" | "zstd" => Box::new(ZstdDecoder::new(BufReader::new(input_file))?),
        // Not a compressed file, return a path to the original
        _ => {
            return Ok(DecompressedImage {
                path: input_path.to_path_buf(),
                _temp_handle: None,
            });
        }
    };

    let decompress_pb = ProgressBar::new_spinner();
    decompress_pb.set_prefix("Decompress");
    // A custom spinner animation for decompression
    decompress_pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&[
                &style("■  ■  ■  ■  ■  ■  ■                           ")
                    .blue()
                    .to_string(),
                &style(" ■  ■  ■  ■  ■  ■  ■                          ")
                    .blue()
                    .to_string(),
                &style("  ■  ■  ■  ■  ■  ■  ■                         ")
                    .blue()
                    .to_string(),
                &style("   ■  ■  ■  ■  ■  ■  ■                        ")
                    .blue()
                    .to_string(),
                &style("    ■  ■  ■  ■  ■  ■  ■                       ")
                    .blue()
                    .to_string(),
                &style("     ■  ■  ■  ■  ■  ■  ■                      ")
                    .blue()
                    .to_string(),
                &style("      ■  ■  ■  ■  ■  ■  ■                     ")
                    .blue()
                    .to_string(),
                &style("       ■  ■  ■  ■  ■  ■  ■                    ")
                    .blue()
                    .to_string(),
                &style("        ■  ■  ■  ■  ■  ■  ■                   ")
                    .blue()
                    .to_string(),
                &style("         ■  ■  ■  ■  ■  ■  ■                  ")
                    .blue()
                    .to_string(),
                &style("          ■  ■  ■  ■  ■  ■  ■                 ")
                    .blue()
                    .to_string(),
                &style("           ■  ■  ■  ■  ■  ■  ■                ")
                    .blue()
                    .to_string(),
                &style("            ■  ■  ■  ■  ■  ■  ■               ")
                    .blue()
                    .to_string(),
                &style("             ■  ■  ■  ■  ■  ■  ■              ")
                    .blue()
                    .to_string(),
                &style("              ■  ■  ■  ■  ■  ■  ■             ")
                    .blue()
                    .to_string(),
                &style("               ■  ■  ■  ■  ■  ■  ■            ")
                    .blue()
                    .to_string(),
                &style("                ■  ■  ■  ■  ■  ■  ■           ")
                    .blue()
                    .to_string(),
                &style("                 ■  ■  ■  ■  ■  ■  ■          ")
                    .blue()
                    .to_string(),
                &style("                  ■  ■  ■  ■  ■  ■  ■         ")
                    .blue()
                    .to_string(),
                &style("                   ■  ■  ■  ■  ■  ■  ■        ")
                    .blue()
                    .to_string(),
                &style("                    ■  ■  ■  ■  ■  ■  ■       ")
                    .blue()
                    .to_string(),
                &style("                     ■  ■  ■  ■  ■  ■  ■      ")
                    .blue()
                    .to_string(),
                &style("                      ■  ■  ■  ■  ■  ■  ■     ")
                    .blue()
                    .to_string(),
                &style("                       ■  ■  ■  ■  ■  ■  ■    ")
                    .blue()
                    .to_string(),
                &style("                        ■  ■  ■  ■  ■  ■  ■   ")
                    .blue()
                    .to_string(),
                &style("                         ■  ■  ■  ■  ■  ■  ■  ")
                    .blue()
                    .to_string(),
                &style("                          ■  ■  ■  ■  ■  ■  ■ ")
                    .blue()
                    .to_string(),
                &style("                           ■  ■  ■  ■  ■  ■  ■")
                    .blue()
                    .to_string(),
                &style("                          ■  ■  ■  ■  ■  ■  ■ ")
                    .blue()
                    .to_string(),
                &style("                         ■  ■  ■  ■  ■  ■  ■  ")
                    .blue()
                    .to_string(),
                &style("                        ■  ■  ■  ■  ■  ■  ■   ")
                    .blue()
                    .to_string(),
                &style("                       ■  ■  ■  ■  ■  ■  ■    ")
                    .blue()
                    .to_string(),
                &style("                      ■  ■  ■  ■  ■  ■  ■     ")
                    .blue()
                    .to_string(),
                &style("                     ■  ■  ■  ■  ■  ■  ■      ")
                    .blue()
                    .to_string(),
                &style("                    ■  ■  ■  ■  ■  ■  ■       ")
                    .blue()
                    .to_string(),
                &style("                   ■  ■  ■  ■  ■  ■  ■        ")
                    .blue()
                    .to_string(),
                &style("                  ■  ■  ■  ■  ■  ■  ■         ")
                    .blue()
                    .to_string(),
                &style("                 ■  ■  ■  ■  ■  ■  ■          ")
                    .blue()
                    .to_string(),
                &style("                ■  ■  ■  ■  ■  ■  ■           ")
                    .blue()
                    .to_string(),
                &style("               ■  ■  ■  ■  ■  ■  ■            ")
                    .blue()
                    .to_string(),
                &style("              ■  ■  ■  ■  ■  ■  ■             ")
                    .blue()
                    .to_string(),
                &style("             ■  ■  ■  ■  ■  ■  ■              ")
                    .blue()
                    .to_string(),
                &style("            ■  ■  ■  ■  ■  ■  ■               ")
                    .blue()
                    .to_string(),
                &style("           ■  ■  ■  ■  ■  ■  ■                ")
                    .blue()
                    .to_string(),
                &style("          ■  ■  ■  ■  ■  ■  ■                 ")
                    .blue()
                    .to_string(),
                &style("         ■  ■  ■  ■  ■  ■  ■                  ")
                    .blue()
                    .to_string(),
                &style("        ■  ■  ■  ■  ■  ■  ■                   ")
                    .blue()
                    .to_string(),
                &style("       ■  ■  ■  ■  ■  ■  ■                    ")
                    .blue()
                    .to_string(),
                &style("      ■  ■  ■  ■  ■  ■  ■                     ")
                    .blue()
                    .to_string(),
                &style("     ■  ■  ■  ■  ■  ■  ■                      ")
                    .blue()
                    .to_string(),
                &style("    ■  ■  ■  ■  ■  ■  ■                       ")
                    .blue()
                    .to_string(),
                &style("   ■  ■  ■  ■  ■  ■  ■                        ")
                    .blue()
                    .to_string(),
                &style("  ■  ■  ■  ■  ■  ■  ■                         ")
                    .blue()
                    .to_string(),
                &style(" ■  ■  ■  ■  ■  ■  ■                          ")
                    .blue()
                    .to_string(),
            ])
            .template("{prefix} [{elapsed_precise}] [{spinner}] {bytes} ({bytes_per_sec}) {msg}")
            .unwrap(),
    );
    decompress_pb.enable_steady_tick(Duration::from_millis(100));

    // Decompress to a named temp file
    let mut temp_file = NamedTempFile::new()?;
    {
        let mut writer = BufWriter::new(&mut temp_file);
        let mut buffer = [0u8; 8192];
        let mut total: u64 = 0;

        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            writer.write_all(&buffer[..n])?;
            total += n as u64;
            decompress_pb.set_position(total);
        }
        writer.flush()?;
    }

    decompress_pb.set_style(
        indicatif::ProgressStyle::with_template(
            "Decompress [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes} ({bytes_per_sec}) {msg}",
        )
        .unwrap()
        .progress_chars("■■"),
    );

    // Ensure the progress bar finishes at 100%
    if let Some(len) = decompress_pb.length() {
        decompress_pb.set_position(len);
    }

    decompress_pb.finish_with_message("✅ Decompression complete.");

    // Hand over ownership of the temp file to the DecompressedImage struct
    let temp_path = temp_file.into_temp_path();
    Ok(DecompressedImage {
        path: temp_path.to_path_buf(),
        _temp_handle: Some(temp_path),
    })
}

pub fn run(image_path: &Path, device_path: &Path, verify: bool) -> Result<()> {
    println!(
        "Writing image \"{}\" to device \"{}\"",
        image_path.display(),
        device_path.display()
    );

    let image = decompress_image(image_path)?;
    let mut image_file = File::open(&image)?;
    let image_len = image_file.metadata()?.len();

    let mut device_file = std::fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_DIRECT) // Use O_DIRECT for unbuffered I/O
        .open(device_path)?;

    let write_pb = make_progress_bar(image_len, "Writing", "green");
    let start_time = Instant::now();

    // Align buffer to 512 bytes for O_DIRECT compatibility
    let block_size = 512;
    let mut buf = vec![0u8; BUFFER_SIZE + block_size];
    let offset = buf.as_ptr().align_offset(block_size);
    let buffer = &mut buf[offset..offset + BUFFER_SIZE];

    let mut written: u64 = 0;
    while written < image_len {
        let to_read = std::cmp::min(BUFFER_SIZE as u64, image_len - written) as usize;
        image_file.read_exact(&mut buffer[..to_read])?;

        // Ensure the data chunk is a multiple of the block size
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
        let mut image_file = File::open(&image)?;
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
