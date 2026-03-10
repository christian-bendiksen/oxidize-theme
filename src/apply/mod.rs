//! Desktop apply steps — GNOME settings, app reloads, wallpaper cycling.

pub mod gnome;
pub mod reload;
pub mod wallpaper;

#[derive(Debug, Clone, Copy)]
pub struct ApplyFlags {
    pub no_apply: bool,
    pub no_gnome: bool,
    pub no_icons: bool,
    pub no_reload: bool,
    pub no_wallpaper: bool,
}
