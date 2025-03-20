# Limine Qemu Cargo Runner

## Installation
- Git version: run `install.sh` or `cargo install --path .`
- Release version: `cargo install cargo-qemu-runner`
## Usage
- Put `runner = "cargo qemu-runner"` in your `.cargo/config.toml`'s `[target]` section
- Specify these keys under `[package.metadata.qemu_runner]` section in `Cargo.toml`:
    - `limine-branch`, the limine binary branch to clone
		- **NOTE: Only branches starting from v4.x are supported**
	- `config-file`, specified path to the limine config file to use
    - `extra-files`, specifies extra files to copy to the resulting image
    - `test-success-exit-code`, qemu exit code which is considered success
    - `test-args`, extra arguments which are used along with `run-command` for tests
    - `run-args`, extra arguments which are used along with `run-command`
    - `run-command`, base command used to run qemu
- Now you can `cargo run` your kernel and it will automatically launch it in qemu

[Example config](https://github.com/Qwinci/cargo-qemu-runner/blob/main/example/Cargo.toml)

## Todo
- More configuration options?
- Make it possible to have `cargo-qemu-runner` as a dev-dependency instead of requiring installation if possible
