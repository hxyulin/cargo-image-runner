# CLAUDE.md

## Build & Test

```bash
cargo build                  # build with default features
cargo test                   # run all tests
cargo build --no-default-features --features "uefi,limine,iso,qemu"  # selective features
```

**Feature flags** (default: `uefi`, `bios`, `limine`, `iso`, `qemu`):
- Boot types: `bios`, `uefi` (pulls `ovmf-prebuilt`)
- Bootloaders: `limine` (pulls `git2`), `grub`
- Image formats: `iso` (pulls `hadris-iso`), `fat` (pulls `fatfs`/`fscommon`)
- Runners: `qemu`
- Optional: `progress` (pulls `indicatif`)

**CI**: Runs `cargo build && cargo test` on stable, beta, nightly (see `.github/workflows/ci.yml`).

**Edition**: Rust 2024 — requires nightly or recent stable.

## Architecture

Pipeline: **Bootloader → ImageBuilder → Runner**, each a trait in the corresponding module.

Entry point is the **builder pattern** (`builder()` → `ImageRunnerBuilder` → `ImageRunner`). Config is read from `[package.metadata.image-runner]` in the target crate's `Cargo.toml` via `cargo_metadata`.

**Context** (`core::Context`) carries state through the pipeline: config, paths, template variables, test detection.

## Module Map

| Module | Role |
|---|---|
| `core/` | `Context`, `ImageRunnerBuilder`, `ImageRunner`, `Error`/`Result` |
| `config/` | `Config` struct, `ConfigLoader` (reads `[package.metadata.image-runner]` from Cargo.toml) |
| `bootloader/` | `Bootloader` trait + impls: `limine`, `grub`, `none`; `fetcher` for downloading bootloader files |
| `image/` | `ImageBuilder` trait + impls: `directory`, `iso`, `fat`; `template` processor |
| `runner/` | `Runner` trait + impl: `qemu` |
| `firmware/` | UEFI firmware (`ovmf`) |
| `util/` | Filesystem helpers (`fs`), hashing (`hash`) |

## Key Patterns

**Template processing** (`image::template::TemplateProcessor`): Substitutes `{{VAR}}` and `$VAR` in config files. Built-in vars: `EXECUTABLE`, `EXECUTABLE_NAME`, `WORKSPACE_ROOT`, `OUTPUT_DIR`, `IS_TEST`. User vars come from `[package.metadata.image-runner.variables]`.

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

**Feature-gated compilation**: Bootloader/image/runner implementations are behind `#[cfg(feature = "...")]`. Adding a new impl means adding a feature flag and the corresponding module.

**Build artifacts** go to `target/image-runner/` (`cache/` for downloads, `output/` for built images).

## CLI Subcommands

`cargo-image-runner [run] <EXECUTABLE>`, `build <EXECUTABLE>`, `clean`, `check`, `version`.
