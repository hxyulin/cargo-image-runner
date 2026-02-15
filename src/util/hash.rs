use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Compute a hash of a file's contents.
pub fn hash_file(path: &Path) -> std::io::Result<u64> {
    let content = std::fs::read(path)?;
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    Ok(hasher.finish())
}

/// Compute a hash of multiple file paths and their contents.
pub fn hash_files(paths: &[&Path]) -> std::io::Result<u64> {
    let mut hasher = DefaultHasher::new();

    for path in paths {
        // Hash the path itself
        path.to_string_lossy().hash(&mut hasher);

        // Hash the content if file exists
        if path.exists() {
            let content = std::fs::read(path)?;
            content.hash(&mut hasher);
        }
    }

    Ok(hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_file_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, b"hello world").unwrap();

        let h1 = hash_file(&file).unwrap();
        let h2 = hash_file(&file).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_file_different_content() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = dir.path().join("a.txt");
        let f2 = dir.path().join("b.txt");
        std::fs::write(&f1, b"hello").unwrap();
        std::fs::write(&f2, b"world").unwrap();

        let h1 = hash_file(&f1).unwrap();
        let h2 = hash_file(&f2).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_files_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = dir.path().join("a.txt");
        let f2 = dir.path().join("b.txt");
        std::fs::write(&f1, b"hello").unwrap();
        std::fs::write(&f2, b"world").unwrap();

        let h1 = hash_files(&[f1.as_path(), f2.as_path()]).unwrap();
        let h2 = hash_files(&[f1.as_path(), f2.as_path()]).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_file_not_found() {
        let result = hash_file(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }
}
