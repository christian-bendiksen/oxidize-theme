//! Filesystem utility functions used across the crate.

use anyhow::{Context, Result};
use std::{fs, path::Path};

/// Remove a path regardless of whether it is a file, symlink, or directory.
pub fn remove_any(p: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(p).with_context(|| format!("stat {}", p.display()))?;

    if meta.is_dir() {
        fs::remove_dir_all(p)
    } else {
        fs::remove_file(p)
    }
    .with_context(|| format!("remove {}", p.display()))
}

/// Create (or replace) a Unix symlink atomically.
///
/// Ensures parent directories exist, removes any existing entry at `link`,
/// then creates `link -> target`.
#[cfg(unix)]
pub fn symlink_force(target: &Path, link: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;

    // Ensure parent exists.
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create parent dir {}", parent.display()))?;
    }

    // Remove whatever is currently at `link`, if anything.
    if fs::symlink_metadata(link).is_ok() {
        remove_any(link).with_context(|| format!("remove existing entry at {}", link.display()))?;
    }

    symlink(target, link)
        .with_context(|| format!("symlink {} -> {}", link.display(), target.display()))
}

#[cfg(not(unix))]
pub fn symlink_force(_target: &Path, _link: &Path) -> Result<()> {
    anyhow::bail!("symlinks are not supported on this platform")
}
