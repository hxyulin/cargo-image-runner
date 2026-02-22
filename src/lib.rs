//! cargo-image-runner: A generic, highly customizable embedded/kernel development runner for Rust.
//!
//! This library provides a flexible framework for building and running bootable images with
//! support for multiple bootloaders (Limine, GRUB, none), image formats (ISO, FAT, directory),
//! and boot types (BIOS, UEFI, hybrid).
//!
//! # Quick Start
//!
//! ## Standalone Usage (no `cargo_metadata` or `clap` required)
//!
//! ```no_run
//! use cargo_image_runner::{builder, Config};
//!
//! # fn main() -> cargo_image_runner::Result<()> {
//! let config = Config::from_toml_str(r#"
//!     [boot]
//!     type = "uefi"
//!     [bootloader]
//!     kind = "none"
//!     [image]
//!     format = "directory"
//! "#)?;
//!
//! builder()
//!     .with_config(config)
//!     .workspace_root(".")
//!     .executable("target/x86_64-unknown-none/debug/my-kernel")
//!     .run()
//! # }
//! ```
//!
//! ## Using Cargo.toml Metadata (requires `cargo-metadata` feature)
//!
//! ```no_run
//! # #[cfg(feature = "cargo-metadata")]
//! # fn main() -> cargo_image_runner::Result<()> {
//! use cargo_image_runner::builder;
//!
//! // Load configuration from Cargo.toml and run
//! builder()
//!     .from_cargo_metadata()?
//!     .no_bootloader()
//!     .directory_output()
//!     .qemu()
//!     .run()
//! # }
//! # #[cfg(not(feature = "cargo-metadata"))]
//! # fn main() {}
//! ```
//!
//! ## Configuration in Cargo.toml
//!
//! ```toml
//! [package.metadata.image-runner.boot]
//! type = "uefi"
//!
//! [package.metadata.image-runner.bootloader]
//! kind = "none"
//!
//! [package.metadata.image-runner.image]
//! format = "directory"
//! ```
//!
//! ## With Limine Bootloader
//!
//! ```toml
//! [package.metadata.image-runner.boot]
//! type = "hybrid"  # Supports both BIOS and UEFI
//!
//! [package.metadata.image-runner.bootloader]
//! kind = "limine"
//! config-file = "limine.conf"
//!
//! [package.metadata.image-runner.bootloader.limine]
//! version = "v8.4.0-binary"
//!
//! [package.metadata.image-runner.variables]
//! TIMEOUT = "5"
//! ```
//!
//! Then create `limine.conf`:
//!
//! ```text
//! timeout: {{TIMEOUT}}
//!
//! /My Kernel
//!     protocol: limine
//!     kernel_path: boot():/boot/{{EXECUTABLE_NAME}}
//! ```
//!
//! # I/O Capture & Streaming
//!
//! The [`IoHandler`] trait enables programmatic interaction with QEMU's serial port:
//! - **Capture** serial output for test assertions ([`CaptureHandler`])
//! - **Tee** output to both capture and terminal ([`runner::io::TeeHandler`])
//! - **React** to patterns and send input ([`runner::io::PatternResponder`])
//!
//! ```no_run
//! use cargo_image_runner::{builder, Config, CaptureHandler};
//!
//! # fn main() -> cargo_image_runner::Result<()> {
//! let config = Config::from_toml_str(r#"
//!     [boot]
//!     type = "uefi"
//!     [bootloader]
//!     kind = "none"
//!     [image]
//!     format = "directory"
//! "#)?;
//!
//! let result = builder()
//!     .with_config(config)
//!     .workspace_root(".")
//!     .executable("target/x86_64-unknown-none/debug/my-kernel")
//!     .io_handler(CaptureHandler::new())
//!     .run_with_result()?;
//!
//! if let Some(output) = &result.captured_output {
//!     if let Some(serial) = &output.serial {
//!         println!("Serial output: {serial}");
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! The library is built around three core traits:
//!
//! - [`Bootloader`](bootloader::Bootloader): Prepares bootloader files and configuration
//! - [`ImageBuilder`](image::ImageBuilder): Builds bootable images in various formats
//! - [`Runner`](runner::Runner): Executes images (e.g., in QEMU)
//! - [`IoHandler`](runner::io::IoHandler): Handles I/O from running instances
//!
//! These traits allow easy extensibility for custom bootloaders, image formats, and runners.
//!
//! # Custom Bootloader Example
//!
//! ```no_run
//! use cargo_image_runner::bootloader::{Bootloader, BootloaderFiles, ConfigFile};
//! use cargo_image_runner::config::BootType;
//! use cargo_image_runner::core::{Context, Result};
//!
//! struct MyBootloader;
//!
//! impl Bootloader for MyBootloader {
//!     fn prepare(&self, ctx: &Context) -> Result<BootloaderFiles> {
//!         Ok(BootloaderFiles::new())
//!     }
//!
//!     fn config_files(&self, ctx: &Context) -> Result<Vec<ConfigFile>> {
//!         Ok(Vec::new())
//!     }
//!
//!     fn boot_type(&self) -> BootType {
//!         BootType::Uefi
//!     }
//!
//!     fn name(&self) -> &str {
//!         "MyBootloader"
//!     }
//! }
//! ```
//!
//! # Features
//!
//! - `default` - Enables `cli`, `cargo-metadata`, `uefi`, `bios`, `limine`, `iso`, and `qemu`
//! - `cli` - CLI binary and argument parsing (implies `cargo-metadata`)
//! - `cargo-metadata` - Load config from `Cargo.toml` via `cargo_metadata`
//! - `uefi` - UEFI boot support (includes OVMF firmware)
//! - `bios` - BIOS boot support
//! - `limine` - Limine bootloader (requires git)
//! - `grub` - GRUB bootloader
//! - `iso` - ISO image format
//! - `fat` - FAT filesystem image format
//! - `qemu` - QEMU runner
//! - `progress` - Progress reporting (optional)
//!
//! For standalone library use without `clap` or `cargo_metadata`:
//!
//! ```toml
//! [dependencies]
//! cargo-image-runner = { version = "0.5", default-features = false, features = ["uefi", "qemu"] }
//! ```

pub mod bootloader;
pub mod config;
pub mod core;
pub mod firmware;
pub mod image;
pub mod runner;
pub mod util;

// Re-export commonly used types
pub use crate::core::{Error, ImageRunner, ImageRunnerBuilder, Result};
pub use config::{BootType, BootloaderKind, Config, ImageFormat, SerialConfig, SerialMode};
pub use runner::io::{CaptureHandler, CapturedIo, IoAction, IoHandler};
pub use runner::{CapturedOutput, RunResult};

/// Create a new image runner builder.
///
/// This is the main entry point for the fluent API.
///
/// # Example
///
/// ```no_run
/// use cargo_image_runner::{builder, Config};
///
/// # fn main() -> cargo_image_runner::Result<()> {
/// let config = Config::from_toml_str(r#"
///     [boot]
///     type = "uefi"
///     [bootloader]
///     kind = "none"
///     [image]
///     format = "directory"
/// "#)?;
///
/// builder()
///     .with_config(config)
///     .workspace_root(".")
///     .executable("target/x86_64-unknown-none/debug/my-kernel")
///     .run()
/// # }
/// ```
pub fn builder() -> ImageRunnerBuilder {
    ImageRunnerBuilder::new()
}
