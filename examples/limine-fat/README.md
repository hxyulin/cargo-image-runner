# limine-fat

Limine bootloader with a FAT filesystem image — UEFI boot from a real FAT image.

## Configuration

| Setting | Value |
|---------|-------|
| Boot type | UEFI |
| Bootloader | Limine |
| Image format | FAT |

## Key Concepts

- Combines Limine bootloader with a FAT filesystem image
- Limine and kernel files are written into a FAT image file
- Requires both the `limine` and `fat` feature flags
- Template variables (`{{TIMEOUT}}`, `{{EXECUTABLE_NAME}}`) are substituted in `limine.conf`
- Useful for testing with real filesystem images while using a bootloader

## How to Run

```bash
cargo run
```
