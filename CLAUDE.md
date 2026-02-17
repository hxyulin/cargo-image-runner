# CLAUDE.md

## Build & Test

```bash
cargo build                  # build main crate (default features)
cargo test                   # run all unit + integration tests
cargo build --no-default-features --features "uefi,limine,iso,qemu"  # selective features
```

**Workspace structure**: Cargo workspace with `default-members = ["."]`. Examples under `examples/` are `no_std` kernel stubs targeting `x86_64-unknown-none` — they are workspace members but not built by default.

**Feature flags** (default: `uefi`, `bios`, `limine`, `iso`, `qemu`):
- Boot types: `bios`, `uefi` (pulls `ovmf-prebuilt`)
- Bootloaders: `limine` (pulls `git2`), `grub`
- Image formats: `iso` (pulls `hadris-iso`), `fat` (pulls `fatfs`/`fscommon`)
- Runners: `qemu`
- Optional: `progress` (pulls `indicatif`), `test-harness` (pulls `regex`)

**CI**: Runs `cargo build && cargo test` on stable, beta, nightly + feature combination matrix (see `.github/workflows/ci.yml`).

**Edition**: Rust 2024 — requires nightly or recent stable.

## Architecture

Pipeline: **Bootloader → ImageBuilder → Runner**, each a trait in the corresponding module.

Entry point is the **builder pattern** (`builder()` → `ImageRunnerBuilder` → `ImageRunner`). Config is read from `[package.metadata.image-runner]` in the target crate's `Cargo.toml` via `cargo_metadata`.

**Context** (`core::Context`) carries state through the pipeline: config, paths, template variables, test detection, CLI/env extra args.

**Configuration layering** (later overrides earlier):
1. Built-in defaults
2. Workspace metadata (`[workspace.metadata.image-runner]`)
3. Package metadata (`[package.metadata.image-runner]`)
4. Standalone TOML file (if provided)
5. Profile overlay (`CARGO_IMAGE_RUNNER_PROFILE`)
6. Individual env var overrides (`CARGO_IMAGE_RUNNER_*`)

## Module Map

| Module | Role |
|---|---|
| `core/` | `Context`, `ImageRunnerBuilder`, `ImageRunner`, `Error`/`Result` |
| `config/` | `Config` struct, `ConfigLoader`, `env` module (env var processing, profiles) |
| `config/env.rs` | Reads `CARGO_IMAGE_RUNNER_*` env vars: profile selection, field overrides, template vars, extra QEMU args |
| `bootloader/` | `Bootloader` trait + impls: `limine`, `grub`, `none`; `fetcher` for downloading bootloader files |
| `image/` | `ImageBuilder` trait + impls: `directory`, `iso`, `fat`; `template` processor |
| `runner/` | `Runner` trait + impl: `qemu` |
| `firmware/` | UEFI firmware (`ovmf`) |
| `harness/` | Test harness for parsing sub-test results from serial output (feature-gated: `test-harness`) |
| `util/` | Filesystem helpers (`fs`), hashing (`hash`) |

## Examples

Located under `examples/`, each demonstrating a different configuration combination:

| Example | Boot | Bootloader | Image |
|---|---|---|---|
| `uefi-simple` | UEFI | None | Directory |
| `limine-directory` | Hybrid | Limine | Directory |
| `limine-iso` | Hybrid | Limine | ISO |
| `uefi-fat` | UEFI | None | FAT |
| `limine-fat` | UEFI | Limine | FAT |
| `bios-limine-iso` | BIOS | Limine | ISO |

## Tests

- **Unit tests**: In-module `#[cfg(test)]` blocks in `core/context.rs`, `core/builder.rs`, `config/loader.rs`, `config/mod.rs`, `config/env.rs`, `util/fs.rs`, `util/hash.rs`, `image/template.rs`, `bootloader/mod.rs`, `core/error.rs`
- **Integration tests**: `tests/config_integration.rs`, `tests/builder_pipeline.rs`, `tests/template_integration.rs`
- **Env var tests** use a `Mutex`-based guard pattern (`ENV_LOCK`) to serialize tests that call `set_var`/`remove_var` (unsafe in Rust 2024 edition)

## Key Patterns

**Template processing** (`image::template::TemplateProcessor`): Substitutes `{{VAR}}` and `$VAR` in config files. Built-in vars: `EXECUTABLE`, `EXECUTABLE_NAME`, `WORKSPACE_ROOT`, `OUTPUT_DIR`, `IS_TEST`. User vars come from `[package.metadata.image-runner.variables]` and `CARGO_IMAGE_RUNNER_VAR_*` env vars.

**Configuration** lives in the target crate's `Cargo.toml`:
```toml
[package.metadata.image-runner.boot]
type = "hybrid"                 # bios | uefi | hybrid

[package.metadata.image-runner.bootloader]
kind = "limine"                 # limine | grub | none
config-file = "limine.conf"

[package.metadata.image-runner.image]
format = "directory"            # directory | iso | fat
```

**Profile system**: Profiles defined under `[package.metadata.image-runner.profiles.<name>]`, selected via `CARGO_IMAGE_RUNNER_PROFILE=<name>`. Applied via recursive JSON deep-merge (`config::loader::deep_merge`).

**Environment variable overrides** (`config::env`): `CARGO_IMAGE_RUNNER_QEMU_MEMORY`, `CARGO_IMAGE_RUNNER_BOOT_TYPE`, `CARGO_IMAGE_RUNNER_VERBOSE`, etc. Applied after profile overlay as highest-priority config source.

**QEMU arg layering** (appended in order): config `extra_args` → test/run `extra-args` → `CARGO_IMAGE_RUNNER_QEMU_ARGS` env var → CLI `-- args`.

**Feature-gated compilation**: Bootloader/image/runner implementations are behind `#[cfg(feature = "...")]`. Adding a new impl means adding a feature flag and the corresponding module.

**Build artifacts** go to `target/image-runner/` (`cache/` for downloads, `output/` for built images).

## CLI Subcommands

`cargo-image-runner [run] <EXECUTABLE> [-- <QEMU_ARGS>]`, `build <EXECUTABLE>`, `clean`, `check`, `version`.

The `check` command displays: config values, active profile, env var overrides, env template variables, and QEMU availability/settings.
