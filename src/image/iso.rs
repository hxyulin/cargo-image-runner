use super::ImageBuilder;
use crate::bootloader::FileEntry;
use crate::config::BootType;
use crate::core::context::Context;
use crate::core::error::{Error, Result};
use crate::util::fs::{copy_file, ensure_dir_exists};
use std::path::PathBuf;

#[cfg(feature = "iso")]
use hadris_iso::boot::options::BootOptions;
#[cfg(feature = "iso")]
use hadris_iso::write::InputFiles;

/// ISO image builder using hadris-iso.
///
/// Creates bootable ISO 9660 images with support for both BIOS and UEFI boot.
/// Uses El-Torito for BIOS boot and ESP (EFI System Partition) for UEFI.
pub struct IsoImageBuilder;

impl IsoImageBuilder {
    /// Create a new ISO image builder.
    pub fn new() -> Self {
        Self
    }

    /// Build ISO image from prepared files.
    #[cfg(feature = "iso")]
    fn build_iso(&self, ctx: &Context, files: &[FileEntry]) -> Result<PathBuf> {
        use hadris_iso::joliet::JolietLevel;
        use hadris_iso::read::PathSeparator;
        use hadris_iso::write::options::{BaseIsoLevel, CreationFeatures, FormatOptions};
        use hadris_iso::write::IsoImageWriter;
        use std::fs::File;
        use std::io::BufWriter;

        // Create staging directory
        let staging_dir = ctx.output_dir.join("iso_staging");

        // Clean existing staging directory
        if staging_dir.exists() {
            std::fs::remove_dir_all(&staging_dir)
                .map_err(|e| Error::image_build(format!("Failed to clean staging directory: {}", e)))?;
        }

        ensure_dir_exists(&staging_dir)
            .map_err(|e| Error::image_build(format!("Failed to create staging directory: {}", e)))?;

        // Copy all files to staging directory
        for file in files {
            let dest = staging_dir.join(&file.dest);
            copy_file(&file.source, &dest)
                .map_err(|e| Error::image_build(format!("Failed to copy file to staging: {}", e)))?;
        }

        // Get output path
        let output = self.output_path(ctx);

        // Remove existing ISO if present
        if output.exists() {
            std::fs::remove_file(&output)
                .map_err(|e| Error::image_build(format!("Failed to remove existing ISO: {}", e)))?;
        }

        // Prepare input files for ISO
        let iso_files = self.prepare_iso_files(&staging_dir, files)?;

        // Configure boot options if needed
        let boot_options = self.configure_boot_options(ctx, &staging_dir)?;

        // Configure creation features
        let features = CreationFeatures {
            filenames: BaseIsoLevel::Level2 {
                supports_lowercase: true,
                supports_rrip: false,
            },
            long_filenames: true, // Support long filenames
            joliet: Some(JolietLevel::Level3), // Unicode filename support
            rock_ridge: None, // Not needed for bootloader
            el_torito: boot_options,
            hybrid_boot: None, // TODO: Configure hybrid boot options if needed
        };

        // Configure format options
        let format_options = FormatOptions {
            volume_name: ctx.config.image.volume_label.clone(),
            sector_size: 2048,
            path_seperator: PathSeparator::ForwardSlash,
            features,
        };

        // Create ISO image
        let output_file = File::create(&output)
            .map_err(|e| Error::image_build(format!("Failed to create output file: {}", e)))?;

        let writer = BufWriter::new(output_file);

        // format_new requires Read + Write + Seek
        // We need to open the file in read/write mode
        drop(writer);
        let rw_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&output)
            .map_err(|e| Error::image_build(format!("Failed to open output file: {}", e)))?;

        IsoImageWriter::format_new(rw_file, iso_files, format_options)
            .map_err(|e| Error::image_build(format!("Failed to create ISO: {}", e)))?;

        // Clean up staging directory
        std::fs::remove_dir_all(&staging_dir)
            .map_err(|e| Error::image_build(format!("Failed to clean up staging directory: {}", e)))?;

        Ok(output)
    }

    /// Prepare files for ISO creation from staging directory.
    #[cfg(feature = "iso")]
    fn prepare_iso_files(&self, staging_dir: &std::path::Path, entries: &[FileEntry]) -> Result<InputFiles> {
        use hadris_iso::read::PathSeparator;
        use hadris_iso::write::{File as IsoFile, InputFiles};
        use std::fs;
        use std::sync::Arc;

        let mut iso_files = Vec::new();

        for entry in entries {
            let source_path = staging_dir.join(&entry.dest);

            if source_path.is_file() {
                // Read file contents
                let contents = fs::read(&source_path)
                    .map_err(|e| Error::image_build(format!("Failed to read file {}: {}", source_path.display(), e)))?;

                // Get the relative path as the name
                let name = entry.dest.to_str()
                    .ok_or_else(|| Error::image_build(format!("Invalid path: {:?}", entry.dest)))?
                    .to_string();

                iso_files.push(IsoFile::File {
                    name: Arc::new(name),
                    contents,
                });
            }
        }

        Ok(InputFiles {
            path_separator: PathSeparator::ForwardSlash,
            files: iso_files,
        })
    }

    /// Configure boot options based on boot type.
    #[cfg(feature = "iso")]
    fn configure_boot_options(&self, ctx: &Context, staging_dir: &std::path::Path) -> Result<Option<BootOptions>> {
        use hadris_iso::boot::options::{BootEntryOptions, BootOptions, BootSectionOptions};
        use hadris_iso::boot::{EmulationType, PlatformId};

        match ctx.config.boot.boot_type {
            BootType::Uefi => {
                // UEFI-only boot - no El Torito needed for pure UEFI
                // The EFI directory structure will be handled automatically
                Ok(None)
            }
            BootType::Bios => {
                let boot_image = self.find_boot_image(staging_dir)?;

                if let Some(boot_path) = boot_image {
                    let boot_entry = BootEntryOptions {
                        boot_image_path: boot_path,
                        load_size: None,
                        emulation: EmulationType::NoEmulation,
                        boot_info_table: false,
                        grub2_boot_info: false,
                    };

                    Ok(Some(BootOptions {
                        write_boot_catalog: true,
                        default: boot_entry,
                        entries: vec![],
                    }))
                } else {
                    Ok(None)
                }
            }
            BootType::Hybrid => {
                // BIOS boot as default entry
                let bios_image = self.find_boot_image(staging_dir)?;
                let bios_entry = if let Some(boot_path) = bios_image {
                    BootEntryOptions {
                        boot_image_path: boot_path,
                        load_size: None,
                        emulation: EmulationType::NoEmulation,
                        boot_info_table: false,
                        grub2_boot_info: false,
                    }
                } else {
                    return Ok(None);
                };

                // UEFI boot as additional section entry
                let mut entries = Vec::new();
                if let Some(uefi_path) = self.find_uefi_boot_image(staging_dir)? {
                    entries.push((
                        BootSectionOptions {
                            platform: PlatformId::UEFI,
                        },
                        BootEntryOptions {
                            boot_image_path: uefi_path,
                            load_size: None,
                            emulation: EmulationType::NoEmulation,
                            boot_info_table: false,
                            grub2_boot_info: false,
                        },
                    ));
                }

                Ok(Some(BootOptions {
                    write_boot_catalog: true,
                    default: bios_entry,
                    entries,
                }))
            }
        }
    }

    /// Find BIOS boot image in staging directory.
    #[cfg(feature = "iso")]
    fn find_boot_image(&self, staging_dir: &std::path::Path) -> Result<Option<String>> {
        let candidates = [
            "limine-bios-cd.bin",
            "limine-cd.bin",
            "isolinux/isolinux.bin",
        ];

        for candidate in &candidates {
            let path = staging_dir.join(candidate);
            if path.exists() {
                return Ok(Some(candidate.to_string()));
            }
        }

        Ok(None)
    }

    /// Find UEFI boot image in staging directory for El Torito.
    #[cfg(feature = "iso")]
    fn find_uefi_boot_image(&self, staging_dir: &std::path::Path) -> Result<Option<String>> {
        let path = staging_dir.join("limine-uefi-cd.bin");
        if path.exists() {
            Ok(Some("limine-uefi-cd.bin".to_string()))
        } else {
            Ok(None)
        }
    }

    /// Stub when iso feature is disabled.
    #[cfg(not(feature = "iso"))]
    fn build_iso(&self, _ctx: &Context, _files: &[FileEntry]) -> Result<PathBuf> {
        Err(Error::feature_not_enabled("iso"))
    }
}

impl Default for IsoImageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageBuilder for IsoImageBuilder {
    fn build(&self, ctx: &Context, files: &[FileEntry]) -> Result<PathBuf> {
        self.build_iso(ctx, files)
    }

    fn output_path(&self, ctx: &Context) -> PathBuf {
        if let Some(ref output) = ctx.config.image.output {
            ctx.output_dir.join(output)
        } else {
            ctx.output_dir.join("image.iso")
        }
    }

    fn supported_boot_types(&self) -> &[BootType] {
        // ISO supports both BIOS and UEFI
        &[BootType::Bios, BootType::Uefi, BootType::Hybrid]
    }

    fn name(&self) -> &str {
        "ISO"
    }
}
