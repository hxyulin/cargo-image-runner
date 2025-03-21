# An Example using cargo-qemu-runner

This is an example of using cargo-qemu-runner to run a simple program.

## How to use

1. Install [cargo-qemu-runner](https://github.com/hxyulin/cargo-qemu-runner)
2. Install the `x86_64-unknown-none` target for Rust, using `rustup target add x86_64-unknown-none`. Note that by default, the nightly toolchain is needed,
due to the use of a custom test framework. You can override the toolchain for this project by running `rustup override set nightly`.
3. Run the example using `cargo run`

## How it works

The example is a simple program that prints "Hello, world!" to the serial console.
This example shows how to do run, test, specify command line arguments, and boot type for QEMU using this tool.


