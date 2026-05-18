use std::path::{Path, PathBuf};
use std::sync::RwLock;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathGuardError {
    #[error("path is not allowed: {0}")]
    NotAllowed(PathBuf),
    #[error("path does not exist: {0}")]
    Missing(PathBuf),
    #[error("path is a symlink and writes are blocked: {0}")]
    Symlink(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Validates filesystem paths against an allowlist of base directories.
///
/// Adapters target files outside the app-data directory (e.g. user home for
/// agent global configs, added project directories).  PathGuard supports
/// runtime extension so the allowlist can be updated as projects are added
/// and agent config locations are discovered.
#[derive(Debug)]
pub struct PathGuard {
    allowed_roots: RwLock<Vec<PathBuf>>,
}

impl Clone for PathGuard {
    fn clone(&self) -> Self {
        Self {
            allowed_roots: RwLock::new(self.allowed_roots()),
        }
    }
}

impl PathGuard {
    pub fn new(allowed_roots: Vec<PathBuf>) -> Self {
        let allowed_roots = allowed_roots
            .into_iter()
            .map(|p| canonicalize_lossy(&p))
            .collect();
        Self {
            allowed_roots: RwLock::new(allowed_roots),
        }
    }

    /// Add a new root to the allowlist at runtime.  Duplicates are ignored.
    pub fn allow(&self, root: impl AsRef<Path>) {
        let path = canonicalize_lossy(root.as_ref());
        let mut roots = self
            .allowed_roots
            .write()
            .unwrap_or_else(|e| e.into_inner());
        if !roots.iter().any(|r| r == &path) {
            roots.push(path);
        }
    }

    pub fn allowed_roots(&self) -> Vec<PathBuf> {
        let roots = self.allowed_roots.read().unwrap_or_else(|e| e.into_inner());
        roots.clone()
    }

    /// Validate a path that may not yet exist (e.g. a future write target).
    /// The parent directory must exist and resolve under an allowed root.
    pub fn ensure_writable(&self, path: impl AsRef<Path>) -> Result<PathBuf, PathGuardError> {
        let path = path.as_ref();
        let parent = path
            .parent()
            .ok_or_else(|| PathGuardError::NotAllowed(path.to_path_buf()))?;
        if !parent.exists() {
            return Err(PathGuardError::Missing(parent.to_path_buf()));
        }
        let canonical_parent = parent.canonicalize()?;
        let candidate = canonical_parent.join(
            path.file_name()
                .ok_or_else(|| PathGuardError::NotAllowed(path.to_path_buf()))?,
        );
        if !self.is_within_allowed(&candidate) {
            return Err(PathGuardError::NotAllowed(candidate));
        }
        if let Ok(meta) = std::fs::symlink_metadata(&candidate) {
            if meta.file_type().is_symlink() {
                return Err(PathGuardError::Symlink(candidate));
            }
        }
        Ok(candidate)
    }

    /// Validate an existing path, rejecting symlinks.
    pub fn ensure_existing(&self, path: impl AsRef<Path>) -> Result<PathBuf, PathGuardError> {
        let path = path.as_ref();
        let meta = std::fs::symlink_metadata(path)
            .map_err(|_| PathGuardError::Missing(path.to_path_buf()))?;
        if meta.file_type().is_symlink() {
            return Err(PathGuardError::Symlink(path.to_path_buf()));
        }
        let canonical = path.canonicalize()?;
        if !self.is_within_allowed(&canonical) {
            return Err(PathGuardError::NotAllowed(canonical));
        }
        Ok(canonical)
    }

    fn is_within_allowed(&self, candidate: &Path) -> bool {
        let roots = self.allowed_roots.read().unwrap_or_else(|e| e.into_inner());
        roots.iter().any(|root| candidate.starts_with(root))
    }
}

fn canonicalize_lossy(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_paths_outside_allowed_roots() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let guard = PathGuard::new(vec![dir.path().to_path_buf()]);
        let target = other.path().join("file.txt");
        let err = guard.ensure_writable(&target).unwrap_err();
        assert!(matches!(err, PathGuardError::NotAllowed(_)));
    }

    #[test]
    fn allows_paths_inside_root() {
        let dir = tempfile::tempdir().unwrap();
        let guard = PathGuard::new(vec![dir.path().to_path_buf()]);
        let target = dir.path().join("file.txt");
        let resolved = guard.ensure_writable(&target).unwrap();
        assert!(resolved.starts_with(dir.path().canonicalize().unwrap()));
    }

    #[test]
    fn rejects_traversal_via_dotdot() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let nested = dir.path().join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let guard = PathGuard::new(vec![dir.path().to_path_buf()]);
        // nested/../../<outside file> escapes root after canonicalization.
        let escape = nested.join("..").join("..").join(
            outside
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
        );
        let err = guard.ensure_writable(&escape).unwrap_err();
        assert!(matches!(err, PathGuardError::NotAllowed(_)));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_targets() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let guard = PathGuard::new(vec![dir.path().to_path_buf()]);
        let real = dir.path().join("real.txt");
        std::fs::write(&real, b"hello").unwrap();
        let link = dir.path().join("link.txt");
        symlink(&real, &link).unwrap();
        let err = guard.ensure_writable(&link).unwrap_err();
        assert!(matches!(err, PathGuardError::Symlink(_)));
    }
}
