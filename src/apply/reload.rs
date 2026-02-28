//! Reload running apps after a theme change.

use crate::ctx::Ctx;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

/// Reload waybar, mako, kitty, btop, and XDG portals.
pub fn run(ctx: &Ctx, kitty_rel: Option<&str>) {
    pkill_signal("waybar", "SIGUSR2");

    detach(Command::new("makoctl").arg("reload"));

    // Use spawn (not status) — systemctl can block on D-Bus activation.
    for service in [
        "xdg-desktop-portal.service",
        "xdg-desktop-portal-gtk.service",
    ] {
        detach(Command::new("systemctl").args(["--user", "restart", service]));
    }

    let kitty_conf = ctx.current_link.join(kitty_rel.unwrap_or("kitty.conf"));
    if kitty_conf.is_file() {
        reload_kitty(&kitty_conf);
    }

    pkill_signal("btop", "SIGUSR2");
}

fn pkill_signal(name: &str, signal: &str) {
    let flag = format!("-{signal}");
    detach(Command::new("pkill").args([flag.as_str(), name]));
}

fn detach(cmd: &mut Command) {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok();
}

// Kitty hot-reload
/// Send `set-colors -a <conf>` to every live kitty socket.
fn reload_kitty(conf: &std::path::Path) {
    let conf_str = conf.to_string_lossy();
    for sock in kitty_sockets() {
        let sock_uri = format!("unix:{}", sock.display());
        let success = Command::new("kitty")
            .args(["@", "--to", &sock_uri, "set-colors", "-a", &conf_str])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_or(false, |s| s.success());

        // Stale sockets accumulate when kitty windows close without cleanup.
        if !success {
            let _ = fs::remove_file(&sock);
        }
    }
}

/// Enumerate kitty UNIX sockets under `~/.cache/kitty/`.
///
/// kitty creates `~/.cache/kitty/kitty` plus optional numbered extras
/// like `kitty-1`, `kitty-2`, etc.
fn kitty_sockets() -> Vec<PathBuf> {
    let base = match std::env::var_os("HOME") {
        Some(h) => PathBuf::from(h).join(".cache/kitty"),
        None => return Vec::new(),
    };

    let Ok(rd) = fs::read_dir(&base) else {
        return Vec::new();
    };

    rd.flatten()
        .filter_map(|e| {
            let path = e.path();
            let name = path.file_name()?.to_str()?;
            if (name == "kitty" || name.starts_with("kitty-")) && is_socket(&path) {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(unix)]
fn is_socket(p: &std::path::Path) -> bool {
    use std::os::unix::fs::FileTypeExt;
    fs::symlink_metadata(p)
        .map(|m| m.file_type().is_socket())
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_socket(_: &std::path::Path) -> bool {
    false
}
