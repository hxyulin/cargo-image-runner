# profiles

Demonstrates the profile system, environment variable overrides, and custom QEMU configuration.

## Configuration

| Setting | Value |
|---------|-------|
| Boot type | UEFI |
| Bootloader | None (direct boot) |
| Image format | Directory |
| Base memory | 256 MB |
| Base cores | 1 |

### Profiles

| Profile | Memory | Cores | GUI | Notes |
|---------|--------|-------|-----|-------|
| *(base)* | 256 MB | 1 | default | Minimal configuration |
| `debug` | 1024 MB | 4 | yes | For interactive debugging |
| `ci` | 512 MB | 1 | no | Headless with test timeout |

## Key Concepts

- **Profile selection**: Switch configurations with `CARGO_IMAGE_RUNNER_PROFILE=<name>`
- **Deep merge**: Profile values are recursively merged into the base config — only specified fields are overridden
- **Environment variable overrides**: Individual fields can be overridden at runtime (highest priority)
- **Configuration layering**: defaults < workspace metadata < package metadata < profile overlay < env var overrides

## How to Run

```bash
# Run with base configuration (256 MB, 1 core)
cargo run

# Run with debug profile (1024 MB, 4 cores, GUI)
CARGO_IMAGE_RUNNER_PROFILE=debug cargo run

# Run with CI profile (512 MB, headless)
CARGO_IMAGE_RUNNER_PROFILE=ci cargo run

# Override memory regardless of profile
CARGO_IMAGE_RUNNER_QEMU_MEMORY=2048 cargo run

# Combine profile with env var override
CARGO_IMAGE_RUNNER_PROFILE=debug CARGO_IMAGE_RUNNER_QEMU_MEMORY=8192 cargo run
```
