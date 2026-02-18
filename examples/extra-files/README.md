# extra-files

Demonstrates including extra files in the boot image and using custom template variables with Limine.

## Configuration

| Setting | Value |
|---------|-------|
| Boot type | Hybrid (BIOS + UEFI) |
| Bootloader | Limine |
| Image format | Directory |
| Extra files | `boot-data.txt` |

## Key Concepts

- **Extra files**: The `[package.metadata.image-runner.extra-files]` table maps destination paths to source paths, allowing files to be placed at specific locations in the boot image
- **Custom template variables**: User-defined variables (`TIMEOUT`, `KERNEL_LOG_LEVEL`) are substituted in `limine.conf`
- **Built-in variables**: `{{EXECUTABLE_NAME}}` is automatically set to the kernel binary filename
- **Variable override**: Variables can be overridden at runtime with `CARGO_IMAGE_RUNNER_VAR_<NAME>=<value>`

## How to Run

```bash
# Run with default variables
cargo run

# Override template variables at runtime
CARGO_IMAGE_RUNNER_VAR_TIMEOUT=10 CARGO_IMAGE_RUNNER_VAR_KERNEL_LOG_LEVEL=trace cargo run
```
