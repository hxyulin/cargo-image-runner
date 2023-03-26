# Limine Qemu Cargo Runner

## Prerequisites
- `xorriso` for creating the iso
- `git` for downloading limine
## Usage
- Install it with `install.sh` (or `cargo install --path .`)
- Put `runner = "cargo qemu-runner"` in your `.cargo/config.toml`'s `[target]` section
- Now you can `cargo run` your kernel and it will automatically launch it in qemu
## Todo
- Write a portable `xorriso` replacement in Rust
- More configuration options?
- Make it possible to have `cargo-qemu-runner` as a dev-dependency instead of requiring installation if possible