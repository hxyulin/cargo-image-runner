//! Bootloader trait and built-in implementations (Limine, GRUB, none).

use crate::config::BootType;
use crate::core::context::Context;
use crate::core::error::Result;
use std::collections::HashMap;
use std::path::PathBuf;

// Bootloader implementations
#[cfg(feature = "limine")]
pub mod limine;

pub mod grub;
pub mod none;

#[cfg(feature = "limine")]
mod fetcher;

#[cfg(feature = "limine")]
pub use fetcher::GitFetcher;

/// Bootloader trait for preparing boot files and configuration.
pub trait Bootloader: Send + Sync {
    /// Prepare bootloader files (download, extract, etc.).
    ///
    /// Returns the files that need to be included in the image.
    fn prepare(&self, ctx: &Context) -> Result<BootloaderFiles>;

    /// Get bootloader configuration files to include in image.
    ///
    /// These files may need template processing.
    fn config_files(&self, ctx: &Context) -> Result<Vec<ConfigFile>>;

    /// Process template variables in content.
    ///
    /// Supports both {{VAR}} and $VAR syntax.
    fn process_templates(&self, content: &str, vars: &HashMap<String, String>) -> Result<String> {
        let mut result = content.to_string();

        // Process {{VAR}} syntax
        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        // Process $VAR syntax (only at word boundaries)
        for (key, value) in vars {
            let placeholder = format!("${}", key);
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }

    /// Get required boot type (BIOS/UEFI/both).
    fn boot_type(&self) -> BootType;

    /// Validate configuration for this bootloader.
    fn validate_config(&self, ctx: &Context) -> Result<()> {
        // Check boot type compatibility
        let required = self.boot_type();
        let configured = ctx.config.boot.boot_type;

        match (required, configured) {
            (BootType::Bios, BootType::Uefi) | (BootType::Uefi, BootType::Bios) => {
                return Err(crate::core::error::Error::unsupported(format!(
                    "Bootloader requires {:?} but boot type is configured as {:?}",
                    required, configured
                )));
            }
            _ => {}
        }

        Ok(())
    }

    /// Get a human-readable name for this bootloader.
    fn name(&self) -> &str;
}

/// Files prepared by the bootloader.
#[derive(Debug, Default)]
pub struct BootloaderFiles {
    /// Files for BIOS boot (e.g., boot sector, stage files).
    pub bios_files: Vec<FileEntry>,

    /// Files for UEFI boot (e.g., EFI executables).
    pub uefi_files: Vec<FileEntry>,

    /// Files that go in the root/system area of the image.
    pub system_files: Vec<FileEntry>,
}

impl BootloaderFiles {
    /// Create an empty set of bootloader files.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a BIOS file.
    pub fn add_bios_file(mut self, source: PathBuf, dest: PathBuf) -> Self {
        self.bios_files.push(FileEntry { source, dest });
        self
    }

    /// Add a UEFI file.
    pub fn add_uefi_file(mut self, source: PathBuf, dest: PathBuf) -> Self {
        self.uefi_files.push(FileEntry { source, dest });
        self
    }

    /// Add a system file.
    pub fn add_system_file(mut self, source: PathBuf, dest: PathBuf) -> Self {
        self.system_files.push(FileEntry { source, dest });
        self
    }
}

/// Configuration file that may need processing.
#[derive(Debug, Clone)]
pub struct ConfigFile {
    /// Source path (template).
    pub source: PathBuf,

    /// Destination path in the image.
    pub dest: PathBuf,

    /// Whether this file needs template variable substitution.
    pub needs_template_processing: bool,
}

impl ConfigFile {
    /// Create a new config file entry.
    pub fn new(source: PathBuf, dest: PathBuf) -> Self {
        Self {
            source,
            dest,
            needs_template_processing: false,
        }
    }

    /// Mark this file as needing template processing.
    pub fn with_template_processing(mut self) -> Self {
        self.needs_template_processing = true;
        self
    }
}

/// File entry for inclusion in the image.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Source path on the host filesystem.
    pub source: PathBuf,

    /// Destination path in the image.
    pub dest: PathBuf,
}

impl FileEntry {
    /// Create a new file entry.
    pub fn new(source: PathBuf, dest: PathBuf) -> Self {
        Self { source, dest }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootloader_files_new_is_empty() {
        let files = BootloaderFiles::new();
        assert!(files.bios_files.is_empty());
        assert!(files.uefi_files.is_empty());
        assert!(files.system_files.is_empty());
    }

    #[test]
    fn test_bootloader_files_builder_chain() {
        let files = BootloaderFiles::new()
            .add_bios_file(PathBuf::from("bios.sys"), PathBuf::from("boot/bios.sys"))
            .add_uefi_file(PathBuf::from("bootx64.efi"), PathBuf::from("efi/boot/bootx64.efi"))
            .add_system_file(PathBuf::from("kernel.elf"), PathBuf::from("boot/kernel.elf"));

        assert_eq!(files.bios_files.len(), 1);
        assert_eq!(files.uefi_files.len(), 1);
        assert_eq!(files.system_files.len(), 1);
        assert_eq!(files.bios_files[0].source, PathBuf::from("bios.sys"));
        assert_eq!(files.uefi_files[0].dest, PathBuf::from("efi/boot/bootx64.efi"));
        assert_eq!(files.system_files[0].source, PathBuf::from("kernel.elf"));
    }

    #[test]
    fn test_config_file_template_processing() {
        let cf = ConfigFile::new(PathBuf::from("limine.conf"), PathBuf::from("boot/limine.conf"));
        assert!(!cf.needs_template_processing);

        let cf = cf.with_template_processing();
        assert!(cf.needs_template_processing);
        assert_eq!(cf.source, PathBuf::from("limine.conf"));
        assert_eq!(cf.dest, PathBuf::from("boot/limine.conf"));
    }

    #[test]
    fn test_file_entry_construction() {
        let entry = FileEntry::new(PathBuf::from("/src/kernel"), PathBuf::from("boot/kernel"));
        assert_eq!(entry.source, PathBuf::from("/src/kernel"));
        assert_eq!(entry.dest, PathBuf::from("boot/kernel"));
    }

    #[test]
    fn test_process_templates_default_impl() {
        let bootloader = none::NoneBootloader::new();
        let mut vars = HashMap::new();
        vars.insert("KEY".to_string(), "value".to_string());

        let result = bootloader.process_templates("Hello {{KEY}}", &vars).unwrap();
        assert_eq!(result, "Hello value");
    }
}
