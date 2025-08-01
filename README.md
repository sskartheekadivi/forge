# Forge

A fast, no-nonsense Rust CLI tool to write and read disk images on Linux. Perfect for flashing SD cards, USB drives, and similar devices with progress and verification.

## Features

- Write raw disk images to devices  
- Read full devices to image files  
- Automatic verification after writing (optional skip)  
- Real-time progress bars with speed and ETA  
- Safe, clear error messages  

## Usage

```bash
# Write image to device with verification
sudo forge write image.img /dev/sdX

# Write image skipping verification
sudo forge write image.img /dev/sdX --no-verify

# Read device to image file
sudo forge read /dev/sdX backup.img
````

## Installation

Coming soon via:

- `cargo install forge`
- `.deb` package (from official PPA, eventually)

For now, you can build it from source:
```bash
git clone https://github.com/sskartheekadivi/forge.git
cd forge
cargo build --release
./target/release/forge write <image> <device>  
```

No dependencies beyond Rust and a sane Linux system.

## Planned Features

* Support for compressed images (`.gz`, `.xz`, `.zst`)
* Multi-device flashing (serial and parallel)
* Smarter image reading with size optimization
* Safety checks and device listing
* Official Debian package and GUI frontend

## License

MIT

## Contributing

We welcome contributions — especially bug reports, focused pull requests, and real-world feedback from people flashing actual devices. If you’ve got something to improve, open an issue or send a PR.

## Warning

Forge writes directly to devices. It can and will erase your data if used incorrectly. Always double-check your device paths (`/dev/sdX`) — Forge assumes you know what you're doing.
