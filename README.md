# Cargo Image Runner

A cargo CLI tool / runner that allows building ISO images from your rust executables and running them in qemu.
Currently only supports x86_64 and UEFI booting, and the limine bootloader.

## Dependencies

For cross-platform compatibility, this crate is mosly self-contained, bundling a version of git2 and [hadris-iso](https://github.com/hxyulin/hadris) to create the ISO image.
A version of limine and ovmf are downloaded at runtime, which requires a network connection for the first run.

> Note: A fully bundled version of limine and ovmf is planned for the future.

## Roadmap

- [ ] Add support for other bootloaders (GRUB, etc.)
- [ ] Add support for other architectures (aarch64, etc.)
- [ ] Make bundling git2 optional (use system libssl, libgit2, or use the command line git)
- [ ] Add support for bundling limine (latest stable release) and ovmf (latest stable release), but allow per project overrides

## License

This is forked from the [cargo-image-runner](https://github.com/Qwinci/cargo-image-runner) project, which is licensed under the MIT license.
