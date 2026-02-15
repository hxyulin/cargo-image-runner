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
        use hadris_iso::rrip::RripOptions;
        use hadris_iso::write::options::{BaseIsoLevel, CreationFeatures, FormatOptions};
        use hadris_iso::write::IsoImageWriter;

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

        // Configure boot options before scanning the staging directory, since
        // this may create additional files (e.g. efi-boot.img for UEFI boot).
        let boot_options = self.configure_boot_options(ctx, &staging_dir)?;

        // Build proper directory tree from staging directory.
        // InputFiles::from_path reads the directory recursively and creates the
        // correct File::Directory / File::File tree that hadris-iso expects.
        let iso_files = InputFiles::from_fs(&staging_dir, PathSeparator::ForwardSlash)
            .map_err(|e| Error::image_build(format!("Failed to read staging directory: {}", e)))?;

        // Configure creation features
        let features = CreationFeatures {
            filenames: BaseIsoLevel::Level2 {
                supports_lowercase: true,
                supports_rrip: true,
            },
            long_filenames: true, // Support long filenames
            joliet: Some(JolietLevel::Level3), // Unicode filename support
            rock_ridge: Some(RripOptions::default()), // Preserve original case filenames
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
        // hadris-iso's format_new reads back from the stream during finalization,
        // so the file must be pre-allocated with sufficient space. Estimate based
        // on total staged file sizes plus ISO overhead.
        let total_content_size = Self::calculate_staging_size(&staging_dir);
        // Allocate: content + 512KB for ISO structures, rounded up to sector boundary
        // Extra overhead for RRIP System Use entries in directory records
        let iso_size = ((total_content_size + 1024 * 1024 + 2047) / 2048) * 2048;

        let rw_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&output)
            .map_err(|e| Error::image_build(format!("Failed to create output file: {}", e)))?;
        rw_file.set_len(iso_size)
            .map_err(|e| Error::image_build(format!("Failed to pre-allocate ISO file: {}", e)))?;

        IsoImageWriter::format_new(rw_file, iso_files, format_options)
            .map_err(|e| Error::image_build(format!("Failed to create ISO: {}", e)))?;

        // Clean up staging directory
        std::fs::remove_dir_all(&staging_dir)
            .map_err(|e| Error::image_build(format!("Failed to clean up staging directory: {}", e)))?;

        Ok(output)
    }

    /// Calculate the total size of files in the staging directory.
    #[cfg(feature = "iso")]
    fn calculate_staging_size(staging_dir: &std::path::Path) -> u64 {
        fn walk_dir(dir: &std::path::Path) -> u64 {
            let mut total = 0;
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        total += walk_dir(&path);
                    } else if let Ok(meta) = path.metadata() {
                        total += meta.len();
                    }
                }
            }
            total
        }
        walk_dir(staging_dir)
    }

    /// Configure boot options based on boot type.
    #[cfg(feature = "iso")]
    fn configure_boot_options(&self, ctx: &Context, staging_dir: &std::path::Path) -> Result<Option<BootOptions>> {
        use hadris_iso::boot::options::{BootEntryOptions, BootOptions, BootSectionOptions};
        use hadris_iso::boot::{EmulationType, PlatformId};

        match ctx.config.boot.boot_type {
            BootType::Uefi => {
                // UEFI boot requires an El Torito entry with PlatformId::UEFI
                // pointing to a FAT image containing EFI/BOOT/BOOTX64.EFI
                if let Some(efi_img) = self.create_efi_boot_image(staging_dir)? {
                    let boot_entry = BootEntryOptions {
                        boot_image_path: efi_img.clone(),
                        load_size: None,
                        emulation: EmulationType::NoEmulation,
                        boot_info_table: false,
                        grub2_boot_info: false,
                    };
                    Ok(Some(BootOptions {
                        write_boot_catalog: true,
                        default: boot_entry.clone(),
                        entries: vec![(
                            BootSectionOptions {
                                platform: PlatformId::UEFI,
                            },
                            boot_entry,
                        )],
                    }))
                } else {
                    Ok(None)
                }
            }
            BootType::Bios => {
                let boot_image = self.find_boot_image(staging_dir)?;

                if let Some(boot_path) = boot_image {
                    let boot_entry = BootEntryOptions {
                        boot_image_path: boot_path,
                        load_size: None,
                        emulation: EmulationType::NoEmulation,
                        boot_info_table: true,
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
                        boot_info_table: true,
                        grub2_boot_info: false,
                    }
                } else {
                    return Ok(None);
                };

                // UEFI boot as additional section entry
                // First check for bootloader-provided UEFI image (e.g. limine-uefi-cd.bin),
                // then fall back to creating an embedded FAT image from EFI boot files
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
                } else if let Some(efi_img) = self.create_efi_boot_image(staging_dir)? {
                    entries.push((
                        BootSectionOptions {
                            platform: PlatformId::UEFI,
                        },
                        BootEntryOptions {
                            boot_image_path: efi_img,
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

    /// Create an embedded FAT image containing UEFI boot files for El Torito.
    ///
    /// Scans the staging directory for `efi/boot/bootx64.efi`, creates an in-memory
    /// FAT filesystem containing that file at `EFI/BOOT/BOOTX64.EFI`, and writes the
    /// result to `efi-boot.img` in the staging directory.
    #[cfg(feature = "iso")]
    fn create_efi_boot_image(&self, staging_dir: &std::path::Path) -> Result<Option<String>> {
        use hadris_fat::{FatFsWriteExt, FatVolumeFormatter, FormatOptions};
        use std::io::Cursor;

        // Find EFI boot file (case-insensitive search for common layouts)
        let efi_path = staging_dir.join("efi/boot/bootx64.efi");
        let efi_path = if efi_path.exists() {
            efi_path
        } else {
            let alt = staging_dir.join("EFI/BOOT/BOOTX64.EFI");
            if alt.exists() {
                alt
            } else {
                return Ok(None);
            }
        };

        let efi_data = std::fs::read(&efi_path)
            .map_err(|e| Error::image_build(format!("Failed to read EFI boot file: {}", e)))?;

        // Calculate FAT image size: file size + 1MB overhead, minimum 1MB, sector-aligned
        let fat_size = ((efi_data.len() as u64 + 1024 * 1024 + 511) / 512) * 512;
        let fat_size = fat_size.max(1024 * 1024);

        let mut buffer = vec![0u8; fat_size as usize];

        {
            let cursor = Cursor::new(&mut buffer[..]);
            let options = FormatOptions::new(fat_size).with_label("EFI_BOOT");
            let fs = FatVolumeFormatter::format(cursor, options)
                .map_err(|e| Error::image_build(format!("Failed to format FAT image: {}", e)))?;

            let root = fs.root_dir();
            let efi_dir = fs
                .create_dir(&root, "EFI")
                .map_err(|e| Error::image_build(format!("Failed to create EFI directory: {}", e)))?;
            let boot_dir = fs.create_dir(&efi_dir, "BOOT").map_err(|e| {
                Error::image_build(format!("Failed to create BOOT directory: {}", e))
            })?;

            let file_entry = fs.create_file(&boot_dir, "BOOTX64.EFI").map_err(|e| {
                Error::image_build(format!("Failed to create BOOTX64.EFI: {}", e))
            })?;
            let mut writer = fs.write_file(&file_entry).map_err(|e| {
                Error::image_build(format!("Failed to open BOOTX64.EFI for writing: {}", e))
            })?;
            writer.write(&efi_data).map_err(|e| {
                Error::image_build(format!("Failed to write EFI boot data: {}", e))
            })?;
            writer.finish().map_err(|e| {
                Error::image_build(format!("Failed to finalize BOOTX64.EFI: {}", e))
            })?;
        }

        // Write FAT image to staging directory
        let efi_img_path = staging_dir.join("efi-boot.img");
        std::fs::write(&efi_img_path, &buffer)
            .map_err(|e| Error::image_build(format!("Failed to write efi-boot.img: {}", e)))?;

        Ok(Some("efi-boot.img".to_string()))
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
