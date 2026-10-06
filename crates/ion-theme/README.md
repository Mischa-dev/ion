# ion-theme

Palettes, built-in themes and live theme sources. Qt-free; the `ThemeEngine`
bridge (`crates/ion-app/src/bridge/theme.rs`) feeds the result into the `Theme`
QML singleton, so every surface re-themes together with no restart.

## Theme sources

Chosen with `Theme.source` (from config's `theme.source`):

| Source | What it does |
| --- | --- |
| `builtin` (default) | `Theme.themeName` picks a theme. `auto` (default) is Ion Dark or Ion Light to match the system. |
| `dms` | Reads the palette file DankMaterialShell writes through Ion's matugen template, and re-themes when it changes. |
| `system` | Ion Dark or Ion Light to match the system, with the system accent color (macOS, KDE, …). |
| `manual` | Reads a palette file you maintain (by hand or from Nix) and re-themes when it changes. |

Built-in themes: `ion-dark`, `ion-light`, `catppuccin-mocha`,
`catppuccin-latte`, `nord`. Community themes are palette files dropped into
`~/.config/ion/themes/<id>.toml` and selected by `<id>`; they live-reload too.

Paths: `~/.config/ion` is `$XDG_CONFIG_HOME/ion` when that is set, on Linux and
macOS alike. `Theme.palettePath` overrides the file for `dms` and `manual`
(default `~/.config/ion/dms-palette.toml` and `~/.config/ion/palette.toml`).

If a palette file is missing or invalid, Ion keeps the last good palette,
logs the reason and exposes it as `Theme.error`.

## Palette files

TOML. Only `background`, `text` and `accent` are required; everything else is
derived. `scheme` is inferred from the background when left out.

```toml
name = "Paper"        # optional
scheme = "light"      # optional: "light" | "dark"

[colors]
background = "#fafafa"
text = "#222222"
accent = "#0055cc"
# Optional: surface, surface_raised, surface_hover, border, text_muted,
# on_accent, danger, warning, success
```

`themes/*.toml` are full examples.

## DMS (DankMaterialShell)

`contrib/matugen/ion.toml` is a matugen template mapping Material You roles to
Ion's tokens. Register it with the matugen config DMS runs for user templates
(`~/.config/matugen/config.toml`):

```toml
[templates.ion]
input_path = "/path/to/ion/crates/ion-theme/contrib/matugen/ion.toml"
output_path = "~/.config/ion/dms-palette.toml"
```

then set `theme.source = "dms"`. Every wallpaper or scheme change in DMS
rewrites the file and Ion follows instantly.
