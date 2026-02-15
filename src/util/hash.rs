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
