//! Atomic publish via a temp-dir → rename protocol.
use crate::{ctx::Ctx, util};
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::{Builder, TempDir};

pub struct Transaction {
    stage: TempDir,
    live: PathBuf,
    link: PathBuf,
}

impl Transaction {
    /// Create a fresh staging directory inside `generated/`.
    pub fn begin(ctx: &Ctx) -> Result<Self> {
        fs::create_dir_all(&ctx.generated_dir).context("create generated dir")?;

        let stage = Builder::new()
            .prefix(".stage.")
            .tempdir_in(&ctx.generated_dir)
            .context("create staging dir")?;

        Ok(Self {
            stage,
            live: ctx.live_dir.clone(),
            link: ctx.current_link.clone(),
        })
    }

    /// Path callers write rendered files into.
    #[inline]
    pub fn stage(&self) -> &Path {
        self.stage.path()
    }

    /// Atomically replace `live/` with the staged tree, then update the
    /// `current` symlink.
    pub fn commit(self) -> Result<()> {
        // Surrender TempDir ownership before any fallible operations.
        // This ensures the staging dir is never deleted by Drop on any
        // error path that follows.
        let stage_path = self.stage.keep();

        // Remove the old live tree if it exists.
        if fs::symlink_metadata(&self.live).is_ok() {
            util::remove_any(&self.live).context("remove stale live dir")?;
        }

        // Atomic swap: both paths are inside `generated/` on the same filesystem.
        fs::rename(&stage_path, &self.live).context("rename stage -> live")?;

        // Point the `current` convenience symlink at the new live tree.
        util::symlink_force(&self.live, &self.link).context("update current symlink")
    }
}
