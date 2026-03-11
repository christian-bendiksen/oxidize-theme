//! Template rendering facade.

pub mod engine;
pub mod parser;

use crate::ctx::Ctx;
use anyhow::Result;
use std::{collections::HashMap, path::Path};

/// Render all templates for a theme into `out_dir`.
pub fn render_all(ctx: &Ctx, theme_root: &Path, out_dir: &Path, vars: &HashMap<String, String>) -> Result<()> {
    engine::render_all(&ctx.templates_dir, &ctx.user_templates_dir, theme_root, out_dir, vars)
}
