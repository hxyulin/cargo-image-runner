# cargo-image-runner

A generic, highly customizable embedded/kernel development runner for Rust. Build and run bootable images with support for multiple bootloaders, image formats, and boot types.

## Features

- **Multiple Bootloaders**: Limine, GRUB, or direct boot (no bootloader)
- **Multiple Image Formats**: Directory (for QEMU), ISO (planned), FAT (planned)
- **Multiple Boot Types**: BIOS, UEFI, or hybrid
- **Trait-Based Architecture**: Easy to extend with custom bootloaders, image builders, and runners
- **Builder Pattern API**: Ergonomic, fluent API for programmatic use
- **Template Variables**: Powerful variable substitution in config files
- **Test Integration**: Automatic test detection and execution

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[build-dependencies]
cargo-image-runner = "0.2"
```

Or install as a binary:

```bash
cargo install cargo-image-runner
```

### Basic UEFI Direct Boot

The simplest setup - boots your UEFI executable directly without a bootloader:

**Cargo.toml:**
```toml
[package.metadata.image-runner.boot]
type = "uefi"

[package.metadata.image-runner.bootloader]
kind = "none"

[package.metadata.image-runner.image]
format = "directory"

# Configure as cargo runner
[target.x86_64-unknown-none]
runner = "cargo-image-runner"
```

Then just run:
```bash
cargo run
```

### With Limine Bootloader

For a full bootloader experience with both BIOS and UEFI support:

**Cargo.toml:**
```toml
[package.metadata.image-runner.boot]
type = "hybrid"  # Supports both BIOS and UEFI

[package.metadata.image-runner.bootloader]
kind = "limine"
config-file = "limine.conf"

[package.metadata.image-runner.bootloader.limine]
version = "v8.4.0-binary"  # Use a specific Limine version

[package.metadata.image-runner.image]
format = "directory"

[package.metadata.image-runner.variables]
TIMEOUT = "5"
KERNEL_CMDLINE = "quiet"
```

**limine.conf:**
```
timeout: {{TIMEOUT}}

/My Kernel
    protocol: limine
    kernel_path: boot():/boot/{{EXECUTABLE_NAME}}
    cmdline: {{KERNEL_CMDLINE}}
```

The runner will automatically:
1. Fetch Limine binaries from GitHub (cached)
2. Process template variables in limine.conf
3. Copy your kernel and bootloader files
4. Run in QEMU with UEFI firmware

## Configuration Reference

### Boot Configuration

```toml
[package.metadata.image-runner.boot]
type = "uefi"    # Options: "bios", "uefi", "hybrid"
```

### Bootloader Configuration

#### No Bootloader (Direct Boot)
```toml
[package.metadata.image-runner.bootloader]
kind = "none"
```

#### Limine Bootloader
```toml
[package.metadata.image-runner.bootloader]
kind = "limine"
config-file = "limine.conf"     # Path to your limine.conf
extra-files = []                # Additional files to copy

[package.metadata.image-runner.bootloader.limine]
version = "v8.4.0-binary"       # Git tag from limine repo
```

**Available Limine versions:** Check [Limine releases](https://github.com/limine-bootloader/limine/releases) for tags like `v8.4.0-binary`, `v8.3.0-binary`, etc.

#### GRUB Bootloader
```toml
[package.metadata.image-runner.bootloader]
kind = "grub"
# GRUB support is basic - contributions welcome!
```

### Image Configuration

```toml
[package.metadata.image-runner.image]
format = "directory"            # Options: "directory", "iso", "fat"
output = "custom-name.iso"      # Optional: custom output path
volume-label = "MYKERNEL"       # Optional: volume label (default: "BOOT")
```

**Image formats:**
- `directory` - Creates a directory structure (works with QEMU `fat:rw:`)
- `iso` - ISO 9660 image (planned, not yet implemented)
- `fat` - FAT filesystem image (planned, not yet implemented)

### Runner Configuration

```toml
[package.metadata.image-runner.runner]
kind = "qemu"

[package.metadata.image-runner.runner.qemu]
binary = "qemu-system-x86_64"   # QEMU binary to use
machine = "q35"                  # Machine type
memory = 1024                    # RAM in MB
cores = 1                        # Number of CPU cores
kvm = true                       # Enable KVM acceleration (Linux only)
extra-args = []                  # Additional QEMU arguments
```

### Test Configuration

```toml
[package.metadata.image-runner.test]
success-exit-code = 33          # Exit code that indicates test success
extra-args = [                  # Additional args for test runs
    "-device", "isa-debug-exit,iobase=0xf4,iosize=0x4"
]
timeout = 60                    # Test timeout in seconds
```

### Run Configuration

```toml
[package.metadata.image-runner.run]
extra-args = [                  # Additional args for normal runs
    "-no-reboot",
    "-serial", "stdio"
]
gui = false                     # Use GUI display
```

### Template Variables

Define custom variables for use in bootloader config files:

```toml
[package.metadata.image-runner.variables]
TIMEOUT = "5"
KERNEL_CMDLINE = "quiet loglevel=3"
CUSTOM_VAR = "value"
```

**Built-in variables:**
- `{{EXECUTABLE}}` - Full path to the executable
- `{{EXECUTABLE_NAME}}` - Executable filename
- `{{WORKSPACE_ROOT}}` - Project workspace root
- `{{OUTPUT_DIR}}` - Output directory path
- `{{IS_TEST}}` - "1" if running tests, "0" otherwise

**Syntax:** Use `{{VAR}}` or `$VAR` in your config files.

## CLI Usage

The runner can be used directly from the command line:

```bash
# Run an executable
cargo-image-runner path/to/executable

# Build image without running
cargo-image-runner build path/to/executable

# Check configuration
cargo-image-runner check

# Clean build artifacts
cargo-image-runner clean

# Show version
cargo-image-runner version
```

## Programmatic API

Use cargo-image-runner as a library:

```rust
use cargo_image_runner::builder;

fn main() -> cargo_image_runner::Result<()> {
    builder()
        .from_cargo_metadata()?
        .no_bootloader()
        .directory_output()
        .qemu()
        .run()
}
```

### Custom Bootloader

Implement the `Bootloader` trait to add custom bootloader support:

```rust
use cargo_image_runner::bootloader::{Bootloader, BootloaderFiles, ConfigFile};
use cargo_image_runner::config::BootType;
use cargo_image_runner::core::Context;
use cargo_image_runner::core::Result;

struct MyBootloader;

impl Bootloader for MyBootloader {
    fn prepare(&self, ctx: &Context) -> Result<BootloaderFiles> {
        // Fetch/prepare bootloader files
        let mut files = BootloaderFiles::new();
        // Add files...
        Ok(files)
    }

    fn config_files(&self, ctx: &Context) -> Result<Vec<ConfigFile>> {
        // Return bootloader config files
        Ok(Vec::new())
    }

    fn boot_type(&self) -> BootType {
        BootType::Uefi
    }

    fn name(&self) -> &str {
        "MyBootloader"
    }
}

// Use it
fn main() -> cargo_image_runner::Result<()> {
    builder()
        .from_cargo_metadata()?
        .bootloader(MyBootloader)
        .run()
}
```

## Complete Example

See the `test-limine` directory for a complete working example:

```
test-limine/
├── Cargo.toml          # Package config with image-runner metadata
├── limine.conf         # Limine bootloader config with templates
├── src/
│   └── main.rs         # Minimal stub kernel
└── .cargo/
    └── config.toml     # Cargo runner configuration
```

To run the example:

```bash
cd test-limine
cargo build --target x86_64-unknown-none
CARGO_MANIFEST_PATH=test-limine/Cargo.toml cargo-image-runner run target/x86_64-unknown-none/debug/test-limine
```

## Feature Flags

Minimize dependencies by selecting only the features you need:

```toml
[dependencies]
cargo-image-runner = { version = "0.2", default-features = false, features = ["uefi", "limine", "qemu"] }
```

**Available features:**
- `default` - Enables `uefi`, `bios`, `limine`, `iso`, and `qemu`
- `uefi` - UEFI boot support (includes OVMF firmware fetching)
- `bios` - BIOS boot support
- `limine` - Limine bootloader (requires git)
- `grub` - GRUB bootloader
- `iso` - ISO image format (not yet implemented)
- `fat` - FAT filesystem image format (not yet implemented)
- `qemu` - QEMU runner
- `progress` - Progress reporting (optional, not yet implemented)

## Architecture

cargo-image-runner uses a trait-based architecture with three core abstractions:

### Bootloader Trait
Prepares bootloader files and processes configuration:
```rust
pub trait Bootloader {
    fn prepare(&self, ctx: &Context) -> Result<BootloaderFiles>;
    fn config_files(&self, ctx: &Context) -> Result<Vec<ConfigFile>>;
    fn boot_type(&self) -> BootType;
    fn name(&self) -> &str;
}
```

### ImageBuilder Trait
Builds bootable images in various formats:
```rust
pub trait ImageBuilder {
    fn build(&self, ctx: &Context, files: &[FileEntry]) -> Result<PathBuf>;
    fn output_path(&self, ctx: &Context) -> PathBuf;
    fn supported_boot_types(&self) -> &[BootType];
    fn name(&self) -> &str;
}
```

### Runner Trait
Executes images:
```rust
pub trait Runner {
    fn run(&self, ctx: &Context, image_path: &Path) -> Result<RunResult>;
    fn is_available(&self) -> bool;
    fn name(&self) -> &str;
}
```

## Troubleshooting

### "limine-bios.sys not found"
Make sure you're using a binary release version like `v8.4.0-binary`, not a source version like `v8.4.0`.

### "revspec not found" error
The Limine version must be a valid git tag. Check available versions at https://github.com/limine-bootloader/limine/releases

### QEMU not found
Install QEMU for your platform:
- **macOS:** `brew install qemu`
- **Linux:** `sudo apt install qemu-system-x86` or `sudo dnf install qemu-system-x86`
- **Windows:** Download from https://www.qemu.org/download/

### Missing OVMF firmware
The runner automatically downloads OVMF firmware for UEFI boot. If you see firmware errors, ensure you have internet connectivity and the `uefi` feature is enabled.

## Current Status

**Working:**
- ✅ Core trait-based architecture
- ✅ Configuration loading from Cargo.toml
- ✅ Direct UEFI boot (no bootloader)
- ✅ Limine bootloader with git fetching
- ✅ Hybrid BIOS/UEFI support
- ✅ Template variable substitution
- ✅ Directory image builder
- ✅ QEMU runner with OVMF
- ✅ Test detection and execution
- ✅ CLI interface

**Planned:**
- 🚧 ISO image building
- 🚧 FAT filesystem images
- 🚧 Full GRUB support
- 🚧 Progress reporting
- 🚧 Additional runners (Bochs, VirtualBox)

## Contributing

Contributions are welcome! The architecture is designed for extensibility:

- **Add a bootloader:** Implement the `Bootloader` trait
- **Add an image format:** Implement the `ImageBuilder` trait
- **Add a runner:** Implement the `Runner` trait

## License

MIT

## Credits

- Built on [Limine](https://github.com/limine-bootloader/limine) bootloader
- Uses [OVMF](https://github.com/tianocore/edk2) for UEFI firmware
- Inspired by the Rust OS development community
