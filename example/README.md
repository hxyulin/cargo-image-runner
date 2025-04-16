# An Example using cargo-image-runner

This is an example of using cargo-image-runner in a project.

## How to use

1. Install cargo-image-runner using `cargo install cargo-image-runner` (or if you cloned the repo, run `cargo install --path .` in the root directory)
2. Ensure that you have a target of either `x86_64-unknown-none` or `aarch64-unknown-none` installed. (This project uses the nightly toolchain, so you'll need to install that too)
3. Run the project (specifying the target) `cargo run --target x86_64-unknown-none`
4. You can also run tests `cargo test --target x86_64-unknown-none`

## How it works

The example is a simple program that prints "Hello, world!" to the serial console.
This example shows how to do run, test, specify command line arguments, and boot type for QEMU using this tool.

See [Cargo.toml](Cargo.toml) for notes about each of the fields, and [limine.conf](limine.conf) for the limine configuration file, as well as the [.cargo/config.toml](.cargo/config.toml) file for the runner configuration.
