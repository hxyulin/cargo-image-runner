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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_dir_exists_creates_directory() {
        let dir = tempfile::tempdir().unwrap();
        let new_dir = dir.path().join("subdir");
        assert!(!new_dir.exists());

        ensure_dir_exists(&new_dir).unwrap();
        assert!(new_dir.exists());
        assert!(new_dir.is_dir());
    }

    #[test]
    fn test_ensure_dir_exists_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let new_dir = dir.path().join("subdir");

        ensure_dir_exists(&new_dir).unwrap();
        ensure_dir_exists(&new_dir).unwrap();
        assert!(new_dir.exists());
    }

    #[test]
    fn test_copy_file_success() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("source.txt");
        let dst = dir.path().join("dest.txt");
        std::fs::write(&src, b"hello world").unwrap();

        copy_file(&src, &dst).unwrap();
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "hello world");
    }

    #[test]
    fn test_copy_file_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("source.txt");
        let dst = dir.path().join("a/b/c/dest.txt");
        std::fs::write(&src, b"nested").unwrap();

        copy_file(&src, &dst).unwrap();
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "nested");
    }

    #[test]
    fn test_copy_file_source_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("nonexistent.txt");
        let dst = dir.path().join("dest.txt");

        let result = copy_file(&src, &dst);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }
}
