use super::{Bootloader, BootloaderFiles, ConfigFile};
use crate::config::BootType;
use crate::core::context::Context;
use crate::core::error::Result;

/// No bootloader - direct boot of the executable.
///
/// This bootloader is suitable for UEFI executables that can be booted directly
/// by placing them in the correct EFI directory structure.
pub struct NoneBootloader;

impl NoneBootloader {
    /// Create a new direct boot (no bootloader) instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoneBootloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Bootloader for NoneBootloader {
    fn prepare(&self, ctx: &Context) -> Result<BootloaderFiles> {
        let mut files = BootloaderFiles::new();

        // For UEFI direct boot, copy the executable to EFI/BOOT/BOOTX64.EFI
        if ctx.config.boot.boot_type.needs_uefi() {
            files = files.add_uefi_file(
                ctx.executable.clone(),
                "efi/boot/bootx64.efi".into(),
            );
        }

        Ok(files)
    }

    fn config_files(&self, _ctx: &Context) -> Result<Vec<ConfigFile>> {
        // No config files needed for direct boot
        Ok(Vec::new())
    }

    fn boot_type(&self) -> BootType {
        // Direct boot supports UEFI only by default
        BootType::Uefi
    }

    fn name(&self) -> &str {
        "Direct Boot (no bootloader)"
    }
}
