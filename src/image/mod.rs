//! Image builder trait and built-in implementations (directory, ISO, FAT).

use crate::bootloader::FileEntry;
use crate::config::BootType;
use crate::core::context::Context;
use crate::core::error::Result;
use std::path::PathBuf;

// Image builder implementations
#[cfg(feature = "iso")]
pub mod iso;

#[cfg(feature = "fat")]
pub mod fat;

pub mod directory;

mod template;
pub use template::TemplateProcessor;

/// Image builder trait for creating bootable images.
pub trait ImageBuilder: Send + Sync {
    /// Build the image from prepared files.
    ///
    /// Returns the path to the created image or directory.
    fn build(&self, ctx: &Context, files: &[FileEntry]) -> Result<PathBuf>;

    /// Check if rebuild is needed based on file changes.
    fn needs_rebuild(&self, ctx: &Context) -> Result<bool> {
        // Default implementation: always rebuild
        // Implementations can override with hash-based change detection
        let _ = ctx;
        Ok(true)
    }

    /// Get the output image path or directory.
    fn output_path(&self, ctx: &Context) -> PathBuf;

    /// Get supported boot types for this image format.
    fn supported_boot_types(&self) -> &[BootType];

    /// Clean up old artifacts.
    fn clean(&self, ctx: &Context) -> Result<()> {
        let output = self.output_path(ctx);
        if output.exists() {
            if output.is_dir() {
                std::fs::remove_dir_all(&output)?;
            } else {
                std::fs::remove_file(&output)?;
            }
        }
        Ok(())
    }

    /// Get a human-readable name for this image builder.
    fn name(&self) -> &str;

    /// Validate that the configured boot type is supported.
    fn validate_boot_type(&self, ctx: &Context) -> Result<()> {
        let configured = ctx.config.boot.boot_type;
        let supported = self.supported_boot_types();

        // Check if configured boot type is supported
        let is_supported = supported.iter().any(|&bt| match (bt, configured) {
            // Exact match
            (a, b) if a == b => true,
            // Hybrid supports both BIOS and UEFI
            (BootType::Hybrid, _) | (_, BootType::Hybrid) => true,
            _ => false,
        });

        if !is_supported {
            return Err(crate::core::error::Error::unsupported(format!(
                "{} does not support {:?} boot type (supported: {:?})",
                self.name(),
                configured,
                supported
            )));
        }

        Ok(())
    }
}
