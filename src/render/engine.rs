//! Template rendering engine and TOML variable builder.

use super::parser::{parse, Segment};
use anyhow::{bail, Context, Result};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

// Variable building
pub fn build_vars_from_colors(colors_file: &Path) -> Result<HashMap<String, String>> {
    let src = fs::read_to_string(colors_file)
        .with_context(|| format!("read {}", colors_file.display()))?;

    let table: toml::Value = toml::from_str(&src).context("parse colors.toml")?;

    let mut vars = HashMap::new();
    flatten("", &table, &mut vars);

    // Collect derived keys separately to avoid a borrow conflict on `vars`.
    let derived: Vec<(String, String)> = vars
        .iter()
        .filter(|(_, v)| v.starts_with('#'))
        .flat_map(|(k, v)| derive_color_keys(k, v))
        .collect();

    vars.extend(derived);
    Ok(vars)
}

/// Flatten a TOML value into `prefix_key = string` pairs.
fn flatten(prefix: &str, value: &toml::Value, out: &mut HashMap<String, String>) {
    match value {
        toml::Value::Table(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.to_owned()
                } else {
                    format!("{prefix}_{k}")
                };
                flatten(&key, v, out);
            }
        }
        toml::Value::String(s) => {
            out.insert(prefix.to_owned(), s.clone());
        }
        toml::Value::Integer(i) => {
            out.insert(prefix.to_owned(), i.to_string());
        }
        toml::Value::Float(f) => {
            out.insert(prefix.to_owned(), f.to_string());
        }
        toml::Value::Boolean(b) => {
            out.insert(prefix.to_owned(), b.to_string());
        }
        // Arrays and datetimes are not used in color files — silently ignore.
        _ => {}
    }
}

/// Produce `<key>_strip` and `<key>_rgb` entries from a `#rrggbb` value.
fn derive_color_keys(key: &str, hex: &str) -> impl Iterator<Item = (String, String)> {
    let bare = hex.trim_start_matches('#');
    let rgb = hex_to_rgb(bare).map(|r| (format!("{key}_rgb"), r));
    let strip = (format!("{key}_strip"), bare.to_owned());
    std::iter::once(strip).chain(rgb)
}

/// Convert a bare 6-character hex string to `"r,g,b"`.
fn hex_to_rgb(hex: &str) -> Option<String> {
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(format!("{r},{g},{b}"))
}

// Template rendering
pub fn render_all(
    templates_dir: &Path,
    user_templates_dir: &Path,
    theme_files_dir: &Path,
    out_dir: &Path,
    vars: &HashMap<String, String>,
) -> Result<()> {
    if !templates_dir.is_dir() {
        bail!("templates directory not found: {}", templates_dir.display());
    }
    fs::create_dir_all(out_dir).context("create output directory")?;

    // a hashset bag with claimed items that will not generate with
    // the template enginge.
    let mut claimed: HashSet<PathBuf> = HashSet::new();

    if user_templates_dir.is_dir() {
        for tpl in templates_in(user_templates_dir) {
            let rel = tpl.strip_prefix(user_templates_dir)?.to_path_buf();
            render_one(&tpl, &rel, vars, out_dir)?;
            claimed.insert(rel.with_extension("")); // key = output path (no .tpl)
        }
    }

    if theme_files_dir.is_dir() {
        for src in theme_files_in(theme_files_dir) {
            let rel = src.strip_prefix(theme_files_dir)?.to_path_buf();
            if !claimed.contains(&rel) {
                let out_path = out_dir.join(&rel);
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&src, &out_path)?;
                claimed.insert(rel); // already no extension, key = output path
            }
        }
    }

    for tpl in templates_in(templates_dir) {
        let rel = tpl.strip_prefix(templates_dir)?.to_path_buf();
        if !claimed.contains(&rel.with_extension("")) {
            // compare against output path
            render_one(&tpl, &rel, vars, out_dir)?;
        }
    }

    Ok(())
}

/// Render a single template file to `out_dir / rel` (minus `.tpl` extension).
fn render_one(
    tpl_path: &Path,
    rel: &Path,
    vars: &HashMap<String, String>,
    out_dir: &Path,
) -> Result<()> {
    let src = fs::read_to_string(tpl_path)
        .with_context(|| format!("read template {}", tpl_path.display()))?;

    let rendered = expand(&src, vars);

    let out_path = out_dir.join(&rel.with_extension("")); // strip .tpl

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create output subdir {}", parent.display()))?;
    }
    fs::write(&out_path, rendered).with_context(|| format!("write {}", out_path.display()))
}

/// Expand `{{ key }}` tokens in `src` using `vars`.
///
/// Unknown keys are left as `{{ key }}` so partial renders are inspectable.
/// The output buffer is pre-sized with a single pass to avoid reallocations.
fn expand(src: &str, vars: &HashMap<String, String>) -> String {
    let segments = parse(src);

    let capacity: usize = segments
        .iter()
        .map(|s| match s {
            Segment::Lit(t) => t.len(),
            Segment::Var(k) => vars.get(*k).map_or(k.len() + 6, String::len),
        })
        .sum();

    let mut out = String::with_capacity(capacity);

    for seg in &segments {
        match seg {
            Segment::Lit(t) => out.push_str(t),
            Segment::Var(k) => match vars.get(*k) {
                Some(v) => out.push_str(v),
                None => {
                    out.push_str("{{ ");
                    out.push_str(k);
                    out.push_str(" }}");
                }
            },
        }
    }

    out
}

/// Walk `dir` and yield paths of all `*.tpl` files.
fn templates_in(dir: &Path) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().and_then(|x| x.to_str()) == Some("tpl")
        })
        .map(|e| e.into_path())
}

fn theme_files_in(dir: &Path) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && !is_theme_metadata(e.path()))
        .map(|e| e.into_path())
}

fn is_theme_metadata(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("colors.toml" | "light.mode" | "icons.theme" | "backgrounds")
    )
}
