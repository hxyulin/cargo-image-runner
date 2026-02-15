use super::ImageBuilder;
use crate::bootloader::FileEntry;
use crate::config::BootType;
use crate::core::context::Context;
use crate::core::error::Result;
use crate::util::fs::{copy_file, ensure_dir_exists};
use std::path::PathBuf;

/// Directory-based image builder.
///
/// This builder creates a directory structure suitable for use with QEMU's fat:rw: driver.
/// It's the simplest image format and is ideal for development.
pub struct DirectoryBuilder;

impl DirectoryBuilder {
    /// Create a new directory builder.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DirectoryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageBuilder for DirectoryBuilder {
    fn build(&self, ctx: &Context, files: &[FileEntry]) -> Result<PathBuf> {
        let output = self.output_path(ctx);

        // Clean existing directory
        if output.exists() {
            std::fs::remove_dir_all(&output)?;
        }

        // Create directory structure
        ensure_dir_exists(&output)?;

        // Copy all files
        for file in files {
            let dest = output.join(&file.dest);
            copy_file(&file.source, &dest)?;
        }

        // Also copy the executable if using direct boot
        // (This is handled by the bootloader, but we keep it here for compatibility)

        Ok(output)
    }

    fn output_path(&self, ctx: &Context) -> PathBuf {
        ctx.output_dir.join("esp")
    }

    fn supported_boot_types(&self) -> &[BootType] {
        // Directory output supports all boot types
        &[BootType::Bios, BootType::Uefi, BootType::Hybrid]
    }

    fn name(&self) -> &str {
        "Directory"
    }
}
