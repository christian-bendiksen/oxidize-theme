# oxidize-theme
WIP!

atomic Wayland theme switcher. Sets colors, reloads kitty/waybar/mako/btop, updates wallpaper, applies GNOME settings.

---

## Theme structure

~/.config/oxidize/themes/
├── data/
│   └── <theme-name>/
│       ├── colors.toml        # required
│       ├── light.mode         # optional: marks theme as light
│       ├── icons.theme        # optional: icon theme name
│       ├── backgrounds/       # optional: wallpaper images
│       └── kitty.conf         # optional: any config file
├── templates/                 # *.tpl files, rendered from colors.toml
├── user-templates/            # *.tpl overrides
└── generated/
    └── live/                  # active rendered output (symlinked as current/)

---

## Configuration files

user-templates/<file>.tpl  →  rendered, always wins
data/<theme>/<file>        →  copied if present
templates/<file>.tpl       →  rendered if config is missing

---

## colors.toml

[palette]
bg = "#1e1e2e"
fg = "#cdd6f4"
Produces:
- palette_bg → #1e1e2e
- palette_bg_strip → 1e1e2e
- palette_bg_rgb → 30,30,46
Used in templates as {{ palette_bg }}.
