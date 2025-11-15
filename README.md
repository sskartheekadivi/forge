# etchr

<p align="center">
  </p>

<h3 align="center">A fast, safe, and interactive CLI for flashing disk images.</h3>

<p align="center">
  Tired of cryptic `dd` commands? Worried you'll accidentally wipe your system drive?
  <br />
  <code>etchr</code> is a modern, reliable tool that makes flashing SD cards and USB drives simple and safe, right from your terminal.
</p>

<p align="center">
  
</p>

---

## ✨ Features

* **🛡️ Interactive Safety First**
    `etchr` doesn't let you pass a device path. Instead, it shows an interactive menu of **only removable devices**, making it nearly impossible to flash your system drive by mistake.

* **🚀 Decompression On-the-Fly**
    Automatically decompresses `.gz`, `.xz`, and `.zst` images while writing. No need to extract them first.

* **⚡ Blazingly Fast**
    Optimized for high-speed, unbuffered I/O to flash images as fast as your hardware allows, often faster than GUI-based tools.

* **✅ Guaranteed Verification**
    Automatically verifies the disk with a SHA256 hash after writing to ensure the data is perfect, bit-for-bit. (You can skip this with `--no-verify`).

* **📊 Detailed Progress**
    A beautiful progress bar shows your speed, data transferred, and ETA, so you're never left guessing.

* **🛑 Graceful Cancel**
    Press `Ctrl+C` at any time to safely cancel the operation. `etchr` cleans up after itself, leaving no temporary files or half-written states.

## 🚀 Installation

### 1. With `cargo` (Recommended)
This is the easiest way to get the latest version if you have the Rust toolchain.
```bash
cargo install etchr
```

### 2. From GitHub Releases
Download the pre-compiled binary or `.deb` package from the [Releases page](https://github.com/sskartheekadivi/etchr/releases).
```bash
# For .deb packages
sudo dpkg -i ./etchr_1.0.0_amd64.deb
```

### 3. From Source
```bash
git clone [https://github.com/sskartheekadivi/etchr.git](https://github.com/sskartheekadivi/etchr.git)
cd etchr
cargo build --release
sudo cp ./target/release/etchr /usr/local/bin/
```

## 💡 Usage

`etchr` is designed to be simple. The commands guide you.

### `etchr list`
List all detected removable devices and their mount points.
```
$ etchr list
Found 1 removable devices:

  DEVICE       NAME                 SIZE LOCATION
  ----------   -----------------   ----- ----------
  /dev/sdd     Cruzer Blade       29.5 GB /media/user/USB_DISK
```

### `etchr write`
Write an image to a device. You will be prompted to select a target from a safe, interactive list.
```bash
# You can use compressed or uncompressed images
etchr write ~/Downloads/raspberry-pi-os.img.xz
```
This will start the interactive prompt:
```
✔ Select the target device to WRITE to · /dev/sdd     29.5 GB [Mounted at /media/user/USB_DISK]
WARNING: This will erase all data on 'sdd' (29.5 GB).
  Device: /dev/sdd
  Image:  /home/user/Downloads/raspberry-pi-os.img.xz

✔ Are you sure you want to proceed? · yes

Writing image...
Decompress [■■■■■■■■■■■■■■■■■] 1.53 GiB (150.37 MiB/s)
Writing    [■■■■■■■■■■■■■■■■■] 8.00 GiB (90.12 MiB/s)
Verifying  [■■■■■■■■■■■■■■■■■] 8.00 GiB (133.33 MiB/s)

✨ Successfully flashed /dev/sdd with raspberry-pi-os.img.xz.
```

**Options:**
* `--no-verify`: Skips the verification step after writing.

### `etchr read`
Create an image file by reading an entire device. You will be prompted to select a source.
```bash
etchr read ~/Backups/my-sd-card-backup.img
```
This will start the interactive prompt:
```
✔ Select the source device to READ from · /dev/sdd     29.5 GB [Mounted at /media/user/USB_DISK]
This will read 29.5 GB from 'sdd'.
  Device: /dev/sdd
  Output: /home/user/Backups/my-sd-card-backup.img

✔ Are you sure you want to proceed? · yes

Reading    [■■■■■■■■■■■■■■■■■] 29.5 GiB (100.0 MiB/s)

✨ Successfully read /dev/sdd to my-sd-card-backup.img.
```

## 🗺️ Roadmap

`etchr` is already a powerful tool, but here's what's planned:

* [ ] Smarter reading (e.g., only reading partitions, not the whole empty disk).
* [ ] Multi-write: Flashing one image to multiple devices at once.
* [ ] A (separate) optional GUI frontend.

## Contributing

Contributions are welcome! Whether it's a bug report, a feature idea, or a pull request, feel free to open an issue or start a discussion.

## License
This project is licensed under the MIT License.
