# uefi-simple

Minimal UEFI direct boot example — the simplest possible configuration.

## Configuration

| Setting | Value |
|---------|-------|
| Boot type | UEFI |
| Bootloader | None (direct boot) |
| Image format | Directory |

## Key Concepts

- Direct UEFI boot without any bootloader
- OVMF firmware is automatically downloaded and used
- Directory image format lets QEMU boot directly from a folder via `fat:rw:`
- This is the recommended starting point for new projects

## How to Run

```bash
cargo run
```
