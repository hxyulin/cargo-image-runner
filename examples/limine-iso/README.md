# limine-iso

Limine bootloader with ISO image format — produces a bootable `.iso` file.

## Configuration

| Setting | Value |
|---------|-------|
| Boot type | Hybrid (BIOS + UEFI) |
| Bootloader | Limine |
| Image format | ISO |

## Key Concepts

- Produces a standard ISO 9660 image that can be booted in QEMU or burned to physical media
- Requires the `iso` feature flag (enabled by default)
- Limine is embedded into the ISO with both BIOS and UEFI boot support
- Template variables in `limine.conf` are processed before image creation

## How to Run

```bash
cargo run
```
