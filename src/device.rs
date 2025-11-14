use anyhow::{Result, anyhow};
use dialoguer::{Confirm, Select, theme::ColorfulTheme};
use std::fmt;
use std::fs; // Used for reading /sys/block
use std::io; // Used for error handling on file reads
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Device {
    pub path: PathBuf,
    /// The kernel name of the device (e.g., "sdd").
    pub name: String,
    pub size_gb: f64,
    pub mount_point: String,
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mount_info = if !self.mount_point.is_empty() {
            format!("[Mounted at {}]", self.mount_point)
        } else {
            "[Not mounted]".to_string()
        };

        write!(
            f,
            "{:<15} {:.1} GB {}",
            self.path.display(), // e.g., "/dev/sdd"
            self.size_gb,
            mount_info
        )
    }
}

/// Helper to read a specific file from the /sys/block filesystem.
fn read_sys_file(device_name: &str, file: &str) -> io::Result<String> {
    let path = PathBuf::from("/sys/block").join(device_name).join(file);
    fs::read_to_string(path).map(|s| s.trim().to_string())
}

/// Helper to find the parent device of a partition (e.g., /dev/sda1 -> /dev/sda).
/// This is used to find the system drive's parent for exclusion.
fn get_parent_device_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();

    if path_str.starts_with("/dev/sd") {
        if let Some(index) = path_str.rfind(|c: char| c.is_alphabetic()) {
            return PathBuf::from(&path_str[..=index]);
        }
    } else if path_str.starts_with("/dev/mmcblk") || path_str.starts_with("/dev/nvme") {
        if let Some(index) = path_str.find('p') {
            return PathBuf::from(&path_str[..index]);
        }
    }

    path.to_path_buf()
}

/// Scans for all removable block devices, excluding the main system drive.
pub fn get_removable_devices() -> Result<Vec<Device>> {
    // Use `sysinfo` to find the system drive's parent (e.g., /dev/nvme0n1)
    // so it can be reliably excluded.
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut system_disk_parent = None;
    for disk in disks.iter() {
        if disk.mount_point() == Path::new("/") {
            // e.g., disk.name() is "nvme0n1p2"
            let path = PathBuf::from("/dev/").join(disk.name());
            // system_disk_parent becomes "/dev/nvme0n1"
            system_disk_parent = Some(get_parent_device_path(&path));
            break;
        }
    }
    let system_disk_parent = system_disk_parent
        .ok_or_else(|| anyhow!("Could not determine system drive. Aborting for safety."))?;

    // Iterate over all block devices in /sys/block for reliable detection.
    let mut devices = Vec::new();
    let block_dir = fs::read_dir("/sys/block")?;

    for entry in block_dir.filter_map(Result::ok) {
        let device_name = entry.file_name().to_string_lossy().to_string();
        let device_path = PathBuf::from("/dev/").join(&device_name);

        // Filter 1: Skip loop devices
        if device_name.starts_with("loop") {
            continue;
        }

        // Filter 2: Skip the system drive's parent (e.g., /dev/nvme0n1)
        if device_path == system_disk_parent {
            continue;
        }

        // Filter 3: Check if the kernel flags it as removable.
        // This is the most reliable filter.
        // (e.g., /sys/block/sda/removable == "0")
        // (e.g., /sys/block/sdd/removable == "1")
        let is_removable = read_sys_file(&device_name, "removable")
            .map(|s| s == "1")
            .unwrap_or(false);

        if !is_removable {
            continue; // Will filter out internal drives like /dev/sda
        }

        // Filter 4: Check for 0 size (empty card slots)
        // (e.g., /sys/block/sdb/size == "0")
        let size_sectors = read_sys_file(&device_name, "size")
            .and_then(|s| {
                s.parse::<u64>()
                    .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))
            })
            .unwrap_or(0);

        if size_sectors == 0 {
            continue; // Will filter out empty slots like /dev/sdb, /dev/sdc
        }

        let size_gb = (size_sectors * 512) as f64 / (1024.0 * 1024.0 * 1024.0);

        // Filter 5: Try to find a mount point by checking the `sysinfo` list.
        // `disks` is a list of partitions, so we check if any partition
        // (e.g., "sdd1") starts with the parent device name (e.g., "sdd").
        let mut mount_point = "".to_string();
        for disk in disks.iter() {
            if disk.name().to_string_lossy().starts_with(&device_name) {
                let mp = disk.mount_point().to_string_lossy().to_string();
                if !mp.is_empty() {
                    mount_point = mp;
                    break; // Use the first mount point found
                }
            }
        }

        devices.push(Device {
            path: device_path,
            name: device_name,
            size_gb,
            mount_point,
        });
    }

    Ok(devices)
}

/// Presents an interactive menu for the user to select a device.
pub fn select_device(devices: &[Device], prompt: &str) -> Result<Device> {
    if devices.is_empty() {
        return Err(anyhow!("No removable devices found."));
    }

    let items: Vec<String> = devices.iter().map(|d| d.to_string()).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact()?;

    Ok(devices[selection].clone())
}

/// Presents a final "Yes/No" confirmation to the user.
pub fn confirm_operation(prompt: &str, _device: &Device, _image: &Path) -> Result<bool> {
    let confirmation = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(false)
        .interact()?;

    Ok(confirmation)
}
