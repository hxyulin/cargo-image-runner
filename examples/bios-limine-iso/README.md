# bios-limine-iso

BIOS-only boot with Limine bootloader and ISO image format.

## Configuration

| Setting | Value |
|---------|-------|
| Boot type | BIOS |
| Bootloader | Limine |
| Image format | ISO |

## Key Concepts

- BIOS-only boot — does not use UEFI firmware (no OVMF needed)
- Limine provides legacy BIOS boot support via its BIOS boot stages
- Produces a bootable ISO image with BIOS boot records
- Requires the `bios`, `limine`, and `iso` feature flags
- Template variables in `limine.conf` are processed before image creation

## How to Run

```bash
cargo run
```
