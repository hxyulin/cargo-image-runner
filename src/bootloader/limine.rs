use super::{Bootloader, BootloaderFiles, ConfigFile};
use crate::config::BootType;
use crate::core::context::Context;
use crate::core::error::{Error, Result};
use std::path::PathBuf;

#[cfg(feature = "limine")]
use super::GitFetcher;

/// Limine bootloader implementation.
///
/// Limine is a modern, feature-rich bootloader that supports both BIOS and UEFI.
/// This implementation fetches Limine binaries from the official repository and
/// configures them based on the user's limine.conf file.
pub struct LimineBootloader {
    repo_url: String,
}

impl LimineBootloader {
    /// Limine repository URL.
    const DEFAULT_REPO_URL: &'static str = "https://github.com/limine-bootloader/limine.git";

    /// Create a new Limine bootloader instance.
    pub fn new() -> Self {
        Self {
            repo_url: Self::DEFAULT_REPO_URL.to_string(),
        }
    }

    /// Create a Limine bootloader with a custom repository URL.
    pub fn with_repo_url(repo_url: String) -> Self {
        Self { repo_url }
    }

    /// Get the Limine version from config.
    fn get_version<'a>(&self, ctx: &'a Context) -> &'a str {
        &ctx.config.bootloader.limine.version
    }

    /// Fetch Limine binaries from git.
    #[cfg(feature = "limine")]
    fn fetch_limine(&self, ctx: &Context) -> Result<PathBuf> {
        let version = self.get_version(ctx);
        let cache_dir = ctx.cache_dir.join("bootloaders");

        let fetcher = GitFetcher::new(cache_dir);
        fetcher.fetch_ref(&self.repo_url, "limine", version)
    }

    /// Stub when limine feature is disabled.
    #[cfg(not(feature = "limine"))]
    fn fetch_limine(&self, _ctx: &Context) -> Result<PathBuf> {
        Err(Error::feature_not_enabled("limine"))
    }
}

impl Default for LimineBootloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Bootloader for LimineBootloader {
    fn prepare(&self, ctx: &Context) -> Result<BootloaderFiles> {
        let limine_repo = self.fetch_limine(ctx)?;

        let mut files = BootloaderFiles::new();

        // Prepare BIOS files if needed
        if ctx.config.boot.boot_type.needs_bios() {
            // Copy limine-bios.sys to boot directory
            let limine_bios = limine_repo.join("limine-bios.sys");
            if !limine_bios.exists() {
                return Err(Error::bootloader(
                    "limine-bios.sys not found in Limine repository. \
                     Make sure you're using a binary release (e.g., v8.x-binary)."
                        .to_string(),
                ));
            }

            files = files.add_system_file(limine_bios, "limine-bios.sys".into());

            // CD-specific BIOS boot binary for ISO images
            let limine_bios_cd = limine_repo.join("limine-bios-cd.bin");
            if !limine_bios_cd.exists() {
                return Err(Error::bootloader(
                    "limine-bios-cd.bin not found in Limine repository. \
                     Make sure you're using a binary release (e.g., v8.x-binary)."
                        .to_string(),
                ));
            }
            files = files.add_system_file(limine_bios_cd, "limine-bios-cd.bin".into());
        }

        // Prepare UEFI files if needed
        if ctx.config.boot.boot_type.needs_uefi() {
            // Copy BOOTX64.EFI to EFI/BOOT directory
            let bootx64 = limine_repo.join("BOOTX64.EFI");
            if !bootx64.exists() {
                return Err(Error::bootloader(
                    "BOOTX64.EFI not found in Limine repository. \
                     Make sure you're using a binary release (e.g., v8.x-binary)."
                        .to_string(),
                ));
            }

            files = files.add_uefi_file(bootx64, "efi/boot/bootx64.efi".into());

            // CD-specific UEFI boot binary for ISO images
            let limine_uefi_cd = limine_repo.join("limine-uefi-cd.bin");
            if !limine_uefi_cd.exists() {
                return Err(Error::bootloader(
                    "limine-uefi-cd.bin not found in Limine repository. \
                     Make sure you're using a binary release (e.g., v8.x-binary)."
                        .to_string(),
                ));
            }
            files = files.add_system_file(limine_uefi_cd, "limine-uefi-cd.bin".into());
        }

        // Copy the kernel executable to the boot directory
        files = files.add_system_file(
            ctx.executable.clone(),
            PathBuf::from("boot").join(
                ctx.executable
                    .file_name()
                    .ok_or_else(|| Error::config("invalid executable path"))?,
            ),
        );

        Ok(files)
    }

    fn config_files(&self, ctx: &Context) -> Result<Vec<ConfigFile>> {
        let mut configs = Vec::new();

        // Check for limine.conf in the workspace or specified path
        let config_path = if let Some(ref path) = ctx.config.bootloader.config_file {
            ctx.workspace_root.join(path)
        } else {
            ctx.workspace_root.join("limine.conf")
        };

        if config_path.exists() {
            configs.push(
                ConfigFile::new(config_path, "limine.conf".into())
                    .with_template_processing(),
            );
        } else {
            // Generate a default limine.conf if none exists
            return Err(Error::config(format!(
                "limine.conf not found at {}. Please create a Limine configuration file.",
                config_path.display()
            )));
        }

        // Add any extra files specified in config
        for extra_file in &ctx.config.bootloader.extra_files {
            let src = ctx.workspace_root.join(extra_file);
            if !src.exists() {
                return Err(Error::config(format!(
                    "extra bootloader file not found: {}",
                    src.display()
                )));
            }

            let dest = extra_file
                .file_name()
                .ok_or_else(|| Error::config("invalid extra file path"))?
                .into();

            configs.push(ConfigFile::new(src, dest));
        }

        Ok(configs)
    }

    fn boot_type(&self) -> BootType {
        // Limine supports both BIOS and UEFI
        BootType::Hybrid
    }

    fn name(&self) -> &str {
        "Limine"
    }

    fn validate_config(&self, ctx: &Context) -> Result<()> {
        // Check that version is specified
        let version = self.get_version(ctx);
        if version.is_empty() {
            return Err(Error::config(
                "Limine version not specified in configuration",
            ));
        }

        // Recommend binary releases
        if !version.contains("binary") {
            eprintln!(
                "Warning: Limine version '{}' may require building from source. \
                 Consider using a binary release like 'v8.x-binary' for faster builds.",
                version
            );
        }

        Ok(())
    }
}
