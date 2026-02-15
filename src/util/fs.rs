use std::path::Path;
use std::process::{Command, Stdio};

/// Ensure a directory exists, creating it if necessary.
pub fn ensure_dir_exists(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Copy a file, creating parent directories as needed.
pub fn copy_file(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source file not found: {}", src.display()),
        ));
    }

    if let Some(parent) = dst.parent() {
        ensure_dir_exists(parent)?;
    }

    std::fs::copy(src, dst).map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!("Failed to copy {} to {}: {}", src.display(), dst.display(), e),
        )
    })?;
    Ok(())
}

/// Calculate total size of files in bytes.
pub fn calculate_total_size(files: &[crate::bootloader::FileEntry]) -> std::io::Result<u64> {
    let mut total = 0u64;
    for entry in files {
        if let Ok(metadata) = std::fs::metadata(&entry.source) {
            total += metadata.len();
        }
    }
    Ok(total)
}

/// Check if a command is available on the system.
pub fn check_command_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}
