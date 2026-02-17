# uefi-fat

UEFI direct boot with a FAT filesystem image.

## Configuration

| Setting | Value |
|---------|-------|
| Boot type | UEFI |
| Bootloader | None (direct boot) |
| Image format | FAT |

## Key Concepts

- Produces a FAT filesystem image file instead of a directory
- QEMU boots from the FAT image directly using drive attachment
- Requires the `fat` feature flag (not enabled by default)
- Useful when you need a real filesystem image rather than QEMU's virtual FAT directory

## How to Run

```bash
cargo run
```
