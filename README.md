# Limine Qemu Cargo Runner

## Prerequisites
- `xorriso` for creating the iso
- `git` for downloading limine
## Installation
- Git version: run `install.sh` or `cargo install --path .`
- Release version: `cargo install cargo-qemu-runner`
## Usage
- Put `runner = "cargo qemu-runner"` in your `.cargo/config.toml`'s `[target]` section
- Specify these keys under it:
    - `limine-branch`, the limine binary branch to clone
    - `extra-files`, specifies extra files to copy to the resulting image
    - `test-success-exit-code`, qemu exit code which is considered success
    - `test-args`, extra arguments which are used along with `run-command` for tests
    - `run-args`, extra arguments which are used along with `run-command`
    - `run-command`, base command used to run qemu
- Now you can `cargo run` your kernel and it will automatically launch it in qemu

[Example config](https://github.com/Qwinci/cargo-qemu-runner/blob/main/example/Cargo.toml)

## Todo
- Write a portable `xorriso` replacement in Rust
- More configuration options?
- Make it possible to have `cargo-qemu-runner` as a dev-dependency instead of requiring installation if possible