//! `oxidize-theme` — atomic theme switcher for Wayland desktops.
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod apply;
mod ctx;
mod render;
mod theme;
mod transaction;
mod util;

use ctx::Ctx;
use theme::Theme;
use transaction::Transaction;

#[derive(Parser)]
#[command(name = "oxidize", about = "Atomic Wayland theme switcher")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Set {
        theme: String,
        #[arg(long)]
        no_apply: bool,
        #[arg(long)]
        no_gnome: bool,
        #[arg(long)]
        no_icons: bool,
        #[arg(long)]
        no_reload: bool,
        #[arg(long)]
        no_wallpaper: bool,
    },

    /// Reload apps without changing the theme
    Reload {
        /// Kitty config path relative to themes/current/ (default: kitty.conf)
        #[arg(long)]
        kitty: Option<String>,
    },

    /// Apply GNOME color-scheme and gtk-theme for the current theme
    Gnome {
        #[arg(long)]
        no_icons: bool,
    },

    /// Cycle to the next wallpaper for the current theme
    Wallpaper,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let ctx = Ctx::new().context("initialise context")?;

    match cli.cmd {
        Cmd::Set {
            theme,
            no_apply,
            no_gnome,
            no_icons,
            no_reload,
            no_wallpaper,
        } => cmd_set(
            &ctx,
            &theme,
            apply::ApplyFlags {
                no_apply,
                no_gnome,
                no_icons,
                no_reload,
                no_wallpaper,
            },
        ),

        Cmd::Reload { kitty } => {
            apply::reload::run(&ctx, kitty.as_deref());
            Ok(())
        }

        Cmd::Gnome { no_icons } => {
            let theme = current_theme(&ctx)?;
            apply::gnome::run(&theme, no_icons);
            Ok(())
        }

        Cmd::Wallpaper => {
            let theme = current_theme(&ctx)?;
            apply::wallpaper::run(&ctx, &theme)
        }
    }
}

fn cmd_set(ctx: &Ctx, theme_name: &str, flags: apply::ApplyFlags) -> Result<()> {
    let theme = Theme::load(&ctx.data_dir, theme_name).context("load theme")?;

    // Stage → commit (atomic rename).
    let txn = Transaction::begin(ctx).context("begin transaction")?;
    render::render_all(ctx, txn.stage(), &theme.vars).context("render templates")?;
    stage_assets(&theme, txn.stage()).context("stage assets")?;
    txn.commit().context("commit transaction")?;

    // Persist theme name outside the atomic tree (intentional).
    std::fs::write(&ctx.current_theme_file, format!("{}\n", theme.name))
        .context("write current.theme")?;

    if flags.no_apply {
        return Ok(());
    }

    // Apply steps are best-effort: warn on failure, never abort.
    if !flags.no_gnome {
        apply::gnome::run(&theme, flags.no_icons);
    }
    if !flags.no_reload {
        apply::reload::run(ctx, None);
    }
    if !flags.no_wallpaper {
        if let Err(e) = apply::wallpaper::run(ctx, &theme) {
            eprintln!("warn: wallpaper apply failed: {e:#}");
        }
    }

    Ok(())
}

/// Read the current theme name from disk and load it.
fn current_theme(ctx: &Ctx) -> Result<Theme> {
    let raw = std::fs::read_to_string(&ctx.current_theme_file).unwrap_or_default();
    let name = raw.trim();

    anyhow::ensure!(
        !name.is_empty(),
        "current theme is not set ({})",
        ctx.current_theme_file.display()
    );

    Theme::load(&ctx.data_dir, name).context("load current theme")
}

/// Symlink per-theme assets (marker files, backgrounds) into the stage dir.
fn stage_assets(theme: &Theme, stage: &std::path::Path) -> Result<()> {
    for name in ["light.mode", "icons.theme"] {
        let src = theme.root.join(name);
        if src.is_file() {
            util::symlink_force(&src, &stage.join(name))
                .with_context(|| format!("symlink {name}"))?;
        }
    }

    if let Some(bg) = &theme.backgrounds_dir {
        util::symlink_force(bg, &stage.join("backgrounds")).context("symlink backgrounds")?;
    }

    Ok(())
}
