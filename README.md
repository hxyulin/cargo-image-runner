# cargo-image-runner

A generic, highly customizable embedded/kernel development runner for Rust. Build and run bootable images with support for multiple bootloaders, image formats, and boot types.

## Features

- **Multiple Bootloaders**: Limine, GRUB, or direct boot (no bootloader)
- **Multiple Image Formats**: Directory (for QEMU), ISO, FAT
- **Multiple Boot Types**: BIOS, UEFI, or hybrid
- **Trait-Based Architecture**: Easy to extend with custom bootloaders, image builders, and runners
- **Builder Pattern API**: Ergonomic, fluent API for programmatic use
- **Template Variables**: Powerful variable substitution in config files
- **Test Integration**: Automatic test detection and execution
- **Environment Variable Overrides**: Runtime configuration without editing files
- **Profile System**: Named configuration presets for different workflows
- **CLI Arg Passthrough**: Pass extra QEMU arguments via `--`

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[build-dependencies]
cargo-image-runner = "0.3"
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

All configuration lives under `[package.metadata.image-runner]` in your `Cargo.toml`. Workspace-level defaults can be set under `[workspace.metadata.image-runner]`.

### Boot Configuration

```toml
[package.metadata.image-runner.boot]
type = "uefi"    # Options: "bios", "uefi", "hybrid"
```

| Value | Description |
|-------|-------------|
| `bios` | BIOS boot only |
| `uefi` | UEFI boot only (default) |
| `hybrid` | Both BIOS and UEFI support |

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
config-file = "limine.conf"     # Path to your limine.conf (relative to workspace root)
extra-files = []                # Additional files to copy into the image

[package.metadata.image-runner.bootloader.limine]
version = "v8.x-binary"        # Git tag from limine repo (default: "v8.x-binary")
```

**Available Limine versions:** Check [Limine releases](https://github.com/limine-bootloader/limine/releases) for tags like `v8.4.0-binary`, `v8.3.0-binary`, etc. Always use the `-binary` suffix.

#### GRUB Bootloader
```toml
[package.metadata.image-runner.bootloader]
kind = "grub"

[package.metadata.image-runner.bootloader.grub]
modules = []                    # GRUB modules to include
```

### Image Configuration

```toml
[package.metadata.image-runner.image]
format = "directory"            # Options: "directory", "iso", "fat"
output = "custom-name.iso"      # Optional: custom output path
volume_label = "MYKERNEL"       # Optional: volume label (default: "BOOT")
```

| Format | Description | Requires Feature |
|--------|-------------|-----------------|
| `directory` | Directory structure (works with QEMU `fat:rw:`) | *(always available)* |
| `iso` | ISO 9660 image | `iso` |
| `fat` | FAT filesystem image | `fat` |

### Runner Configuration

```toml
[package.metadata.image-runner.runner]
kind = "qemu"                   # Currently the only runner

[package.metadata.image-runner.runner.qemu]
binary = "qemu-system-x86_64"   # QEMU binary to use
machine = "q35"                  # Machine type
memory = 1024                    # RAM in MB
cores = 1                        # Number of CPU cores
kvm = true                       # Enable KVM acceleration (Linux only)
extra_args = []                  # Additional QEMU arguments (always applied)
```

### Test Configuration

```toml
[package.metadata.image-runner.test]
success-exit-code = 33          # Exit code that indicates test success
extra-args = [                  # Additional args for test runs only
    "-device", "isa-debug-exit,iobase=0xf4,iosize=0x4"
]
timeout = 60                    # Test timeout in seconds
```

Test binaries are automatically detected by examining the executable name for Cargo's hash suffix pattern (e.g., `my-test-a1b2c3d4e5f6a7b8`).

### Run Configuration

```toml
[package.metadata.image-runner.run]
extra-args = [                  # Additional args for normal (non-test) runs only
    "-no-reboot",
    "-serial", "stdio"
]
gui = false                     # Use GUI display
```

### Verbose Output

```toml
verbose = true                  # Enable verbose output (show build progress messages)
```

### Template Variables

Define custom variables for use in bootloader config files:

```toml
[package.metadata.image-runner.variables]
TIMEOUT = "5"
KERNEL_CMDLINE = "quiet loglevel=3"
CUSTOM_VAR = "value"
```

**Built-in variables** (always available, cannot be overridden by config):

| Variable | Description |
|----------|-------------|
| `{{EXECUTABLE}}` | Full path to the executable |
| `{{EXECUTABLE_NAME}}` | Executable filename only |
| `{{WORKSPACE_ROOT}}` | Project workspace root |
| `{{OUTPUT_DIR}}` | Output directory path |
| `{{IS_TEST}}` | `1` if running tests, `0` otherwise |

**Syntax:** Use `{{VAR}}` or `$VAR` in your config files.

**Variable layering** (later overrides earlier):
1. Config variables (`[variables]`)
2. Environment variables (`CARGO_IMAGE_RUNNER_VAR_*`)
3. Built-in variables (always win)

## Environment Variable Overrides

Override any configuration at runtime without editing files. Useful for debugging, CI/CD, and quick experiments.

### Key Config Field Overrides

| Environment Variable | Overrides | Example |
|---------------------|-----------|---------|
| `CARGO_IMAGE_RUNNER_QEMU_BINARY` | QEMU binary path | `qemu-system-aarch64` |
| `CARGO_IMAGE_RUNNER_QEMU_MEMORY` | Memory in MB | `4096` |
| `CARGO_IMAGE_RUNNER_QEMU_CORES` | CPU cores | `4` |
| `CARGO_IMAGE_RUNNER_QEMU_MACHINE` | Machine type | `virt` |
| `CARGO_IMAGE_RUNNER_BOOT_TYPE` | Boot type | `bios`, `uefi`, `hybrid` |
| `CARGO_IMAGE_RUNNER_VERBOSE` | Verbose output | `1`, `true`, `yes` |
| `CARGO_IMAGE_RUNNER_KVM` | KVM acceleration | `1`/`true`/`yes` or `0`/`false`/`no` |

Invalid values are silently ignored (the config file value is kept).

### Extra QEMU Arguments

```bash
# Whitespace-separated, appended to the QEMU command line
CARGO_IMAGE_RUNNER_QEMU_ARGS="-s -S -device virtio-net" cargo run
```

### Template Variables from Environment

```bash
# Set/override template variables: CARGO_IMAGE_RUNNER_VAR_<NAME>=<value>
CARGO_IMAGE_RUNNER_VAR_TIMEOUT=10 CARGO_IMAGE_RUNNER_VAR_DEBUG=1 cargo run
```

The `VAR_` prefix is stripped, so `CARGO_IMAGE_RUNNER_VAR_TIMEOUT=10` sets the template variable `TIMEOUT` to `10`.

## Profile System

Profiles let you define named configuration presets and switch between them with an environment variable.

### Defining Profiles

Add profiles under `[package.metadata.image-runner.profiles.<name>]`:

```toml
[package.metadata.image-runner.boot]
type = "uefi"

[package.metadata.image-runner.runner.qemu]
memory = 1024

# Debug profile: more memory, GDB server, verbose
[package.metadata.image-runner.profiles.debug]
verbose = true

[package.metadata.image-runner.profiles.debug.runner.qemu]
memory = 4096
extra_args = ["-s", "-S"]

[package.metadata.image-runner.profiles.debug.variables]
DEBUG = "1"

# CI profile: no KVM, no GUI
[package.metadata.image-runner.profiles.ci]

[package.metadata.image-runner.profiles.ci.runner.qemu]
kvm = false

[package.metadata.image-runner.profiles.ci.variables]
CI = "1"
```

### Activating a Profile

```bash
CARGO_IMAGE_RUNNER_PROFILE=debug cargo run
```

Profile values are **deep-merged** into the base config:
- Object fields merge recursively (only specified keys are overridden)
- Scalars and arrays are replaced entirely
- Unspecified fields keep their base values

If the profile name doesn't exist, an error is returned listing available profiles.

### Profile Sources

Profiles can be defined at both workspace and package level. Package-level profiles override workspace-level profiles with the same name.

## Configuration Layering

All configuration follows a strict priority order (later overrides earlier):

### Config Values

| Priority | Source |
|----------|--------|
| 1 (lowest) | Built-in defaults |
| 2 | Workspace metadata (`[workspace.metadata.image-runner]`) |
| 3 | Package metadata (`[package.metadata.image-runner]`) |
| 4 | Standalone TOML file (if provided via API) |
| 5 | Profile overlay (`CARGO_IMAGE_RUNNER_PROFILE`) |
| 6 (highest) | Individual env var overrides (`CARGO_IMAGE_RUNNER_*`) |

### QEMU Extra Args (appended in order)

| Priority | Source |
|----------|--------|
| 1 (first) | `extra_args` from `[runner.qemu]` config |
| 2 | `extra-args` from `[test]` or `[run]` (based on mode) |
| 3 | `CARGO_IMAGE_RUNNER_QEMU_ARGS` env var |
| 4 (last) | CLI `-- args` passthrough |

All sources are appended (not replaced), so args from all layers are present.

### Template Variables

| Priority | Source |
|----------|--------|
| 1 (lowest) | Config variables (`[variables]`) |
| 2 | Environment variables (`CARGO_IMAGE_RUNNER_VAR_*`) |
| 3 (highest) | Built-in variables (`EXECUTABLE`, `WORKSPACE_ROOT`, etc.) |

## CLI Usage

```bash
# Run an executable
cargo-image-runner path/to/executable

# Run with explicit subcommand
cargo-image-runner run path/to/executable

# Pass extra QEMU arguments via --
cargo-image-runner run path/to/executable -- -s -S

# Build image without running
cargo-image-runner build path/to/executable

# Check configuration (shows active profile, env overrides, QEMU settings)
cargo-image-runner check

# Clean build artifacts
cargo-image-runner clean

# Show version
cargo-image-runner version
```

### As a Cargo Runner

Configure in `.cargo/config.toml`:

```toml
[target.x86_64-unknown-none]
runner = "cargo-image-runner"
```

Then `cargo run` and `cargo test` work directly.

## Programmatic API

Use cargo-image-runner as a library:

```rust
use cargo_image_runner::builder;

fn main() -> cargo_image_runner::Result<()> {
    builder()
        .from_cargo_metadata()?
        .executable("path/to/kernel")
        .extra_args(vec!["-s".into(), "-S".into()])
        .run()
}
```

### Builder Methods

| Method | Description |
|--------|-------------|
| `.from_cargo_metadata()?` | Load config from Cargo.toml (includes profiles + env overrides) |
| `.from_config_file(path)?` | Load from a standalone TOML file |
| `.with_config(config)` | Set config directly |
| `.executable(path)` | Set the kernel/executable path |
| `.workspace_root(path)` | Set workspace root |
| `.extra_args(vec)` | Set CLI passthrough QEMU args |
| `.no_bootloader()` | Use no bootloader |
| `.limine()` | Use Limine bootloader |
| `.grub()` | Use GRUB bootloader |
| `.directory_output()` | Output as directory |
| `.iso_image()` | Output as ISO |
| `.fat_image()` | Output as FAT image |
| `.qemu()` | Use QEMU runner |
| `.build()?` | Build `ImageRunner` (does not execute) |
| `.run()?` | Build and immediately execute |

### Custom Bootloader

Implement the `Bootloader` trait to add custom bootloader support:

```rust
use cargo_image_runner::bootloader::{Bootloader, BootloaderFiles, ConfigFile};
use cargo_image_runner::config::BootType;
use cargo_image_runner::core::{Context, Result};

struct MyBootloader;

impl Bootloader for MyBootloader {
    fn prepare(&self, ctx: &Context) -> Result<BootloaderFiles> {
        let files = BootloaderFiles::new();
        Ok(files)
    }

    fn config_files(&self, ctx: &Context) -> Result<Vec<ConfigFile>> {
        Ok(Vec::new())
    }

    fn boot_type(&self) -> BootType {
        BootType::Uefi
    }

    fn name(&self) -> &str {
        "MyBootloader"
    }
}

fn main() -> cargo_image_runner::Result<()> {
    cargo_image_runner::builder()
        .from_cargo_metadata()?
        .bootloader(MyBootloader)
        .run()
}
```

## Examples

Located under `examples/`, each demonstrating a different configuration combination:

| Example | Boot | Bootloader | Image | Notes |
|---------|------|------------|-------|-------|
| `uefi-simple` | UEFI | None | Directory | Simplest possible setup |
| `limine-directory` | Hybrid | Limine | Directory | Fast iteration with Limine |
| `limine-iso` | Hybrid | Limine | ISO | Bootable ISO image |
| `uefi-fat` | UEFI | None | FAT | Real FAT filesystem image |
| `limine-fat` | UEFI | Limine | FAT | Limine with FAT image |
| `bios-limine-iso` | BIOS | Limine | ISO | Legacy BIOS boot |
| `profiles` | UEFI | None | Directory | Profile system and env var overrides |
| `extra-files` | Hybrid | Limine | Directory | Extra files and custom template variables |

## Feature Flags

Minimize dependencies by selecting only the features you need:

```toml
[dependencies]
cargo-image-runner = { version = "0.3", default-features = false, features = ["uefi", "limine", "qemu"] }
```

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `uefi` | UEFI boot support | `ovmf-prebuilt` |
| `bios` | BIOS boot support | - |
| `limine` | Limine bootloader | `git2` |
| `grub` | GRUB bootloader | - |
| `iso` | ISO image format | `hadris-iso` |
| `fat` | FAT filesystem image | `fatfs`, `fscommon` |
| `qemu` | QEMU runner | - |
| `progress` | Progress reporting | `indicatif` |

**Default features:** `uefi`, `bios`, `limine`, `iso`, `qemu`

## Architecture

cargo-image-runner uses a trait-based pipeline architecture:

```
Bootloader -> ImageBuilder -> Runner
```

### Core Traits

**Bootloader** - Prepares bootloader files and processes configuration:
```rust
pub trait Bootloader {
    fn prepare(&self, ctx: &Context) -> Result<BootloaderFiles>;
    fn config_files(&self, ctx: &Context) -> Result<Vec<ConfigFile>>;
    fn boot_type(&self) -> BootType;
    fn name(&self) -> &str;
}
```

**ImageBuilder** - Builds bootable images in various formats:
```rust
pub trait ImageBuilder {
    fn build(&self, ctx: &Context, files: &[FileEntry]) -> Result<PathBuf>;
    fn output_path(&self, ctx: &Context) -> PathBuf;
    fn supported_boot_types(&self) -> &[BootType];
    fn name(&self) -> &str;
}
```

**Runner** - Executes images:
```rust
pub trait Runner {
    fn run(&self, ctx: &Context, image_path: &Path) -> Result<RunResult>;
    fn is_available(&self) -> bool;
    fn name(&self) -> &str;
}
```

### Module Map

| Module | Role |
|--------|------|
| `core/` | `Context`, `ImageRunnerBuilder`, `ImageRunner`, `Error`/`Result` |
| `config/` | `Config` struct, `ConfigLoader`, `env` (environment variable processing) |
| `bootloader/` | `Bootloader` trait + impls: `limine`, `grub`, `none`; `fetcher` for downloads |
| `image/` | `ImageBuilder` trait + impls: `directory`, `iso`, `fat`; `template` processor |
| `runner/` | `Runner` trait + impl: `qemu` |
| `firmware/` | UEFI firmware (`ovmf`) |
| `util/` | Filesystem helpers (`fs`), hashing (`hash`) |

### Build Artifacts

All build artifacts go to `target/image-runner/`:
- `cache/` - Downloaded files (Limine binaries, OVMF firmware)
- `output/` - Built images

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

### Profile not found
If you get `profile 'xyz' not found`, check:
1. The profile is defined under `[package.metadata.image-runner.profiles.xyz]`
2. The `CARGO_IMAGE_RUNNER_PROFILE` value matches the profile name exactly

Use `cargo-image-runner check` to see available configuration and active overrides.

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
