# ion-theme

Palettes, built-in themes and live theme sources. Qt-free; the `ThemeEngine`
bridge (`crates/ion-app/src/bridge/theme.rs`) feeds the result into the `Theme`
QML singleton, so every surface re-themes together with no restart.

## Theme sources

Chosen with `theme.source` in Ion's config (`docs/CONFIG.md`), which the
`Theme` QML singleton binds to:

| Source | What it does |
| --- | --- |
| `builtin` (default) | `theme.name` picks a theme. `auto` (default) is Ion Dark or Ion Light to match the system. |
| `dms` | Reads the palette file DankMaterialShell writes through Ion's matugen template, and re-themes when it changes. |
| `system` | Ion Dark or Ion Light to match the system, with the system accent color (macOS, KDE, …). |
| `manual` | Reads a palette file you maintain (by hand or from Nix) and re-themes when it changes. |

Built-in themes: `ion-dark`, `ion-light`, `catppuccin-mocha`,
`catppuccin-latte`, `nord`. Community themes are palette files dropped into
`~/.config/ion/themes/<id>.toml` and selected by `<id>`; they live-reload too.

Paths: `~/.config/ion` is Ion's config directory (`$ION_CONFIG_DIR`, else `$XDG_CONFIG_HOME/ion`), on Linux and
macOS alike. `theme.palette` overrides the file for `dms` and `manual`
(default `~/.config/ion/dms-palette.toml` and `~/.config/ion/palette.toml`).

If a palette file is missing or invalid, Ion keeps the last good palette,
logs the reason and exposes it as `Theme.error`.

## Web pages

`theme.pages` decides what pages see of the theme:

- `match` (default): pages get the theme's light/dark as
  `prefers-color-scheme`, so sites with a dark style use it under a dark theme.
- `system`: pages get the system's light/dark, whatever Ion's theme is.
- `darken`: like `match`, and under a dark theme pages without a dark style
  are darkened by Chromium's auto dark mode (not tinted with the palette).

Per site, `[theme.sites."<site>"]` can switch darkening on or off
(`darken = true | false`) and add CSS (`css = "…"`). A site covers its
subdomains, and the most specific entry wins. `prefers-color-scheme` stays
browser-wide: QtWebEngine has one setting for every page.

`theme.pageControls` (on by default) gives page scrollbars, form controls
(`accent-color`) and text selection the palette's colors. The CSS sits in a
cascade layer, so a page's own styles always win over it.

Changes apply to open pages live. QtWebEngine has no API for a page color
scheme, so `cpp/theme.cpp` in `ion-app` feeds it through the
`QStyleHints::colorSchemeChanged` signal it listens to; see the comment there.

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
