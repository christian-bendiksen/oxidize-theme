//! Theme descriptor — everything we know about a named theme before rendering.

use crate::render::engine::build_vars_from_colors;
use anyhow::{Context, Result, bail};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

/// A fully-loaded theme ready for rendering and applying.
#[derive(Clone, Debug)]
pub struct Theme {
    pub name: String,
    pub root: PathBuf,
    pub vars: HashMap<String, String>,
    pub is_light: bool,
    pub icon_theme: Option<String>,
    pub backgrounds_dir: Option<PathBuf>,
}

impl Theme {
    pub fn load(data_dir: &Path, name: &str) -> Result<Self> {
        let root = data_dir.join(name);
        if !root.is_dir() {
            bail!("theme not found: {}", root.display());
        }

        let colors_file = root.join("colors.toml");
        if !colors_file.is_file() {
            bail!(
                "missing colors.toml in theme '{name}': {}",
                colors_file.display()
            );
        }

        let vars = build_vars_from_colors(&colors_file)
            .with_context(|| format!("build vars for theme '{name}'"))?;

        let bg_dir = root.join("backgrounds");

        Ok(Self {
            name: name.to_owned(),
            is_light: root.join("light.mode").is_file(),
            icon_theme: read_trimmed(&root.join("icons.theme"))?,
            backgrounds_dir: bg_dir.is_dir().then_some(bg_dir),
            root,
            vars,
        })
    }
}

/// Read a file, trim whitespace, and return `None` if absent or empty.
fn read_trimmed(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(s) => {
            let t = s.trim();
            Ok((!t.is_empty()).then(|| t.to_owned()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("read {}", path.display())),
    }
}
