# Cargo Image Runner

A cargo CLI tool / runner that allows building ISO images from your rust executables and running them in qemu.
Currently only supports x86_64 and UEFI booting, and the limine bootloader.

## Dependencies

For cross-platform compatibility, this crate is mosly self-contained, bundling a version of git2 and [hadris-iso](https://github.com/hxyulin/hadris) to create the ISO image.
A version of limine and ovmf are downloaded at runtime, which requires a network connection for the first run.

> Note: A fully bundled version of limine and ovmf is planned for the future.

## Configuration
The runner provides default arguments for booting limine, however, it is required for the user to create a limine.conf file in the root directory of the project.
This can be overwritten with the `config_file` option, to point to another path, note that the file should be named either 'limine.cfg' for older versions or 'limine.conf', as its filename will be used when copying to the ISO root.

|Key|Values|Description|
|--|--|--|
|config-file|path to file||
|extra-files|list of paths to files|This can be used to copy additional files, such as `initrd` or something similar, or other modules needed for your kernel|
|limine-branch|the branch of limine to use|This should be the binary version, e.g. `v8.x-binary`|
|run-command|list of strings|This is the base command used to run the image, for most projects, this would be `["qemu-system-x86_64", "-cdrom", "{}"]`|
|test-args|list of strings|This is additional arguments to the run command when testing|
|run-args|list of strings|This is additional arguments to the run command when running (not testing)|
|test-success-exit-code|integer|This is the test success exit code, for qemu `33` should be used|
|boot-type|either 'bios' or 'uefi'|This is how the kernel should be booted, either using BIOS or UEFI|
|cmdline|string|This is the commandline passed to limine, and passed to your kernel|
|vars|map of strings to strings|See below|

### Variables
Variables are supported using the `$name` syntax, where they can be used in any of `run-command`, `test-args`, or `run-args`. For example, to make it easy to test on multple machines, you could insert this into `run-command`: `"-machine", "$machine"`, and add machine as a variable. This can be later overwritten using the commandline using a key=value syntax. Other configuration values that can be changed include limine-branch, config-file, and boot-type.

## Roadmap

- [ ] Add support for other bootloaders (GRUB, etc.)
- [ ] Add support for other architectures (aarch64, etc.)
- [ ] Make bundling git2 optional (use system libssl, libgit2, or use the command line git)
    - [x] optional bundling git2 (using cmdline)
    - [ ] support dynamic linking libssl and libgit2
- [ ] Add support for bundling limine (latest stable release) and ovmf (latest stable release), but allow per project overrides

## License

This is forked from the [cargo-qemu-runner](https://github.com/Qwinci/cargo-qemu-runner) project, which is licensed under the MIT license.
