use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("Failed to canonicalize path: {0}")]
    Canonicalization(#[from] std::io::Error),
}

/// Canonicalizes a path using `dunce` to avoid the `\\?\` prefix on Windows.
pub fn canonicalize_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, PathError> {
    dunce::canonicalize(path).map_err(PathError::Canonicalization)
}
