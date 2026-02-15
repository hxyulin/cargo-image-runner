use crate::core::error::{Error, Result};
use std::path::{Path, PathBuf};

/// OVMF firmware manager.
pub struct OvmfFirmware {
    cache_dir: PathBuf,
}

impl OvmfFirmware {
    /// Create a new OVMF firmware manager.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Fetch OVMF firmware if not already cached.
    #[cfg(feature = "uefi")]
    pub fn fetch(&self) -> Result<OvmfFiles> {
        use ovmf_prebuilt::{Arch, FileType, Prebuilt, Source};

        std::fs::create_dir_all(&self.cache_dir)?;

        let prebuilt = Prebuilt::fetch(Source::LATEST, &self.cache_dir)
            .map_err(|e| Error::firmware(format!("failed to fetch OVMF: {}", e)))?;

        let code = prebuilt.get_file(Arch::X64, FileType::Code).to_path_buf();
        let vars = prebuilt.get_file(Arch::X64, FileType::Vars).to_path_buf();

        Ok(OvmfFiles { code, vars })
    }

    /// Fetch OVMF firmware (stub when uefi feature is disabled).
    #[cfg(not(feature = "uefi"))]
    pub fn fetch(&self) -> Result<OvmfFiles> {
        Err(Error::feature_not_enabled("uefi"))
    }
}

/// OVMF firmware files.
#[derive(Debug, Clone)]
pub struct OvmfFiles {
    /// Path to OVMF code file.
    pub code: PathBuf,

    /// Path to OVMF vars file.
    pub vars: PathBuf,
}

impl OvmfFiles {
    /// Get the code file path.
    pub fn code(&self) -> &Path {
        &self.code
    }

    /// Get the vars file path.
    pub fn vars(&self) -> &Path {
        &self.vars
    }
}
