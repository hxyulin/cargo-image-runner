use super::ImageBuilder;
use crate::bootloader::FileEntry;
use crate::config::BootType;
use crate::core::context::Context;
use crate::core::error::{Error, Result};
use std::path::PathBuf;

#[cfg(feature = "fat")]
use std::fs::{File, OpenOptions};
#[cfg(feature = "fat")]
use std::io::Write;

/// FAT filesystem image builder.
///
/// Creates bootable FAT32 filesystem images using the fatfs crate for both
/// formatting and file operations. Pure Rust implementation with no external
/// dependencies. Primarily used for UEFI boot.
pub struct FatImageBuilder;

impl FatImageBuilder {
    /// Create a new FAT image builder.
    pub fn new() -> Self {
        Self
    }

    /// Build FAT image from prepared files.
    #[cfg(feature = "fat")]
    fn build_fat(&self, ctx: &Context, files: &[FileEntry]) -> Result<PathBuf> {
        use fatfs::{format_volume, FormatVolumeOptions};

        // Calculate required image size
        let size = Self::calculate_image_size(files)?;

        // Get output path
        let output = self.output_path(ctx);

        // Remove existing image if present
        if output.exists() {
            std::fs::remove_file(&output)
                .map_err(|e| Error::image_build(format!("Failed to remove existing FAT image: {}", e)))?;
        }

        // Create empty file with calculated size
        let file = File::create(&output)
            .map_err(|e| Error::image_build(format!("Failed to create FAT image file: {}", e)))?;

        file.set_len(size)
            .map_err(|e| Error::image_build(format!("Failed to set FAT image size: {}", e)))?;

        drop(file);

        // Open file for formatting
        let img_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&output)
            .map_err(|e| Error::image_build(format!("Failed to open FAT image: {}", e)))?;

        // Format the image as FAT32
        // Convert volume label to fixed 11-byte array (FAT volume label requirement)
        let volume_label = &ctx.config.image.volume_label;
        let mut label_bytes = [b' '; 11]; // Pad with spaces
        let label_str = volume_label.as_bytes();
        let copy_len = label_str.len().min(11);
        label_bytes[..copy_len].copy_from_slice(&label_str[..copy_len]);

        let format_options = FormatVolumeOptions::new()
            .volume_label(label_bytes);

        format_volume(&img_file, format_options)
            .map_err(|e| Error::image_build(format!("Failed to format FAT image: {}", e)))?;

        drop(img_file);

        // Populate with files
        Self::populate_fat_image(&output, files)?;

        Ok(output)
    }

    /// Populate the FAT image with files using fatfs crate.
    #[cfg(feature = "fat")]
    fn populate_fat_image(image_path: &std::path::Path, files: &[FileEntry]) -> Result<()> {
        use fatfs::{FileSystem, FsOptions};
        use fscommon::BufStream;

        let img_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(image_path)
            .map_err(|e| Error::image_build(format!("Failed to open FAT image: {}", e)))?;

        let buf_stream = BufStream::new(img_file);
        let fs = FileSystem::new(buf_stream, FsOptions::new())
            .map_err(|e| Error::image_build(format!("Failed to open FAT filesystem: {}", e)))?;

        let root_dir = fs.root_dir();

        for file_entry in files {
            // Create parent directories if needed
            Self::create_parent_dirs_fat(&root_dir, &file_entry.dest)?;

            // Get destination path as string
            let dest_str = file_entry
                .dest
                .to_str()
                .ok_or_else(|| Error::image_build(format!("Invalid destination path: {:?}", file_entry.dest)))?;

            // Open source file
            let mut src = File::open(&file_entry.source).map_err(|e| {
                Error::image_build(format!(
                    "Failed to open source file {}: {}",
                    file_entry.source.display(),
                    e
                ))
            })?;

            // Create destination file in FAT image
            let mut dst = root_dir.create_file(dest_str).map_err(|e| {
                Error::image_build(format!("Failed to create file {} in FAT image: {}", dest_str, e))
            })?;

            // Copy contents
            std::io::copy(&mut src, &mut dst).map_err(|e| {
                Error::image_build(format!("Failed to copy file {} to FAT image: {}", dest_str, e))
            })?;

            // Flush to ensure data is written
            dst.flush()
                .map_err(|e| Error::image_build(format!("Failed to flush file {}: {}", dest_str, e)))?;
        }

        Ok(())
    }

    /// Create parent directories in FAT filesystem.
    #[cfg(feature = "fat")]
    fn create_parent_dirs_fat(root: &fatfs::Dir<impl fatfs::ReadWriteSeek>, path: &std::path::Path) -> Result<()> {
        use std::path::Path;

        let parent = path.parent();
        if let Some(parent_path) = parent {
            if parent_path != Path::new("") {
                // Split the path into components and create each directory
                let components: Vec<_> = parent_path.components().collect();

                let mut current_path = String::new();
                for component in components {
                    if let std::path::Component::Normal(name) = component {
                        if !current_path.is_empty() {
                            current_path.push('/');
                        }
                        current_path.push_str(name.to_str().ok_or_else(|| {
                            Error::image_build(format!("Invalid directory name: {:?}", name))
                        })?);

                        // Try to create directory, ignore error if it already exists
                        let _ = root.create_dir(&current_path);
                    }
                }
            }
        }
        Ok(())
    }

    /// Calculate required image size based on files to include.
    #[cfg(feature = "fat")]
    fn calculate_image_size(files: &[FileEntry]) -> Result<u64> {
        let mut total = 0u64;

        for entry in files {
            let metadata = std::fs::metadata(&entry.source).map_err(|e| {
                Error::image_build(format!(
                    "Failed to get metadata for {}: {}",
                    entry.source.display(),
                    e
                ))
            })?;
            total += metadata.len();
        }

        // Add 50% overhead for FAT tables and slack space
        // Minimum 32MB to ensure enough space for boot structures
        let size_with_overhead = (total * 3 / 2).max(32 * 1024 * 1024);

        Ok(size_with_overhead)
    }

    /// Stub when fat feature is disabled.
    #[cfg(not(feature = "fat"))]
    fn build_fat(&self, _ctx: &Context, _files: &[FileEntry]) -> Result<PathBuf> {
        Err(Error::feature_not_enabled("fat"))
    }
}

impl Default for FatImageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageBuilder for FatImageBuilder {
    fn build(&self, ctx: &Context, files: &[FileEntry]) -> Result<PathBuf> {
        self.build_fat(ctx, files)
    }

    fn output_path(&self, ctx: &Context) -> PathBuf {
        if let Some(ref output) = ctx.config.image.output {
            ctx.output_dir.join(output)
        } else {
            ctx.output_dir.join("image.fat")
        }
    }

    fn supported_boot_types(&self) -> &[BootType] {
        // FAT supports UEFI primarily, but can work with hybrid
        &[BootType::Uefi, BootType::Hybrid]
    }

    fn name(&self) -> &str {
        "FAT"
    }
}
