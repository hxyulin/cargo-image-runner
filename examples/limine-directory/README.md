# limine-directory

Limine bootloader with directory image format — hybrid BIOS/UEFI boot.

## Configuration

| Setting | Value |
|---------|-------|
| Boot type | Hybrid (BIOS + UEFI) |
| Bootloader | Limine |
| Image format | Directory |

## Key Concepts

- Limine bootloader is automatically fetched from GitHub and cached
- Hybrid boot type supports both BIOS and UEFI firmware
- Template variables (`{{TIMEOUT}}`, `{{EXECUTABLE_NAME}}`) are substituted in `limine.conf`
- Directory format is the fastest to build, ideal for development iteration

## How to Run

```bash
cargo run
```
