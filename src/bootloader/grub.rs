use super::{Bootloader, BootloaderFiles, ConfigFile};
use crate::config::BootType;
use crate::core::context::Context;
use crate::core::error::Result;

/// GRUB bootloader.
///
/// This is a basic GRUB support implementation. Full GRUB support will be
/// implemented in Phase 2.
pub struct GrubBootloader;

impl GrubBootloader {
    /// Create a new GRUB bootloader instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for GrubBootloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Bootloader for GrubBootloader {
    fn prepare(&self, _ctx: &Context) -> Result<BootloaderFiles> {
        // TODO: Implement GRUB preparation in Phase 2
        // This will involve:
        // - Finding GRUB binaries
        // - Creating GRUB image for BIOS
        // - Preparing GRUB EFI for UEFI
        Ok(BootloaderFiles::new())
    }

    fn config_files(&self, _ctx: &Context) -> Result<Vec<ConfigFile>> {
        // TODO: Implement GRUB config in Phase 2
        // This will process grub.cfg with template variables
        Ok(Vec::new())
    }

    fn boot_type(&self) -> BootType {
        // GRUB supports hybrid boot
        BootType::Hybrid
    }

    fn name(&self) -> &str {
        "GRUB"
    }
}
