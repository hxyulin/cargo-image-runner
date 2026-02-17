//! cargo-image-runner: A generic, highly customizable embedded/kernel development runner for Rust.
//!
//! This library provides a flexible framework for building and running bootable images with
//! support for multiple bootloaders (Limine, GRUB, none), image formats (ISO, FAT, directory),
//! and boot types (BIOS, UEFI, hybrid).
//!
//! # Quick Start
//!
//! ## Using the Builder API
//!
//! ```no_run
//! use cargo_image_runner::builder;
//!
//! # fn main() -> cargo_image_runner::Result<()> {
//! // Load configuration from Cargo.toml and run
//! builder()
//!     .from_cargo_metadata()?
//!     .no_bootloader()
//!     .directory_output()
//!     .qemu()
//!     .run()
//! # }
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
//! # Architecture
//!
//! The library is built around three core traits:
//!
//! - [`Bootloader`](bootloader::Bootloader): Prepares bootloader files and configuration
//! - [`ImageBuilder`](image::ImageBuilder): Builds bootable images in various formats
//! - [`Runner`](runner::Runner): Executes images (e.g., in QEMU)
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
//! - `default` - Enables `uefi`, `bios`, `limine`, `iso`, and `qemu`
//! - `uefi` - UEFI boot support (includes OVMF firmware)
//! - `bios` - BIOS boot support
//! - `limine` - Limine bootloader (requires git)
//! - `grub` - GRUB bootloader
//! - `iso` - ISO image format (not yet implemented)
//! - `fat` - FAT filesystem image format (not yet implemented)
//! - `qemu` - QEMU runner
//! - `progress` - Progress reporting (optional, not yet implemented)

pub mod bootloader;
pub mod config;
pub mod core;
pub mod firmware;
#[cfg(feature = "test-harness")]
pub mod harness;
pub mod image;
pub mod runner;
pub mod util;

// Re-export commonly used types
pub use crate::core::{Error, ImageRunner, ImageRunnerBuilder, Result};
pub use config::{BootType, BootloaderKind, Config, ImageFormat};

/// Create a new image runner builder.
///
/// This is the main entry point for the fluent API.
///
/// # Example
///
/// ```no_run
/// use cargo_image_runner::builder;
///
/// # fn main() -> cargo_image_runner::Result<()> {
/// builder()
///     .from_cargo_metadata()?
///     .no_bootloader()
///     .directory_output()
///     .qemu()
///     .run()
/// # }
/// ```
pub fn builder() -> ImageRunnerBuilder {
    ImageRunnerBuilder::new()
}
