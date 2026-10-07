# Configuration

Ion reads two TOML files from its config directory: `$ION_CONFIG_DIR`, else
`$XDG_CONFIG_HOME/ion`, else `~/.config/ion` (also on macOS, so one
home-manager module covers both platforms).

| File | Written by | Notes |
|---|---|---|
| `config.toml` | home-manager (`programs.ion`) or you | Base layer. Ion never writes it. |
| `overrides.toml` | Ion's settings UI | Wins over the base. Delete a key to fall back. |

Tables merge key by key; any other value in `overrides.toml` replaces the base
value whole (a list is replaced, not appended to). Both files are watched and
re-applied as soon as they change. A file that fails to parse or does not fit
the schema is reported and Ion keeps the last config that loaded. Unknown keys
are only warnings.

## Keys

Keys are camelCase so they match the Nix option names exactly. Everything is
optional; these are the defaults.

```toml
[general]
homePage = "https://duckduckgo.com/"
restoreSession = true

[search]
engine = "DuckDuckGo"
template = "https://duckduckgo.com/?q={}"

[ui]
density = "comfortable"      # "comfortable" | "compact"
cornerRadius = 8
tabs = "horizontal"          # "horizontal" | "vertical"
animations = { enable = true, speed = 1.0 }

[theme]
source = "builtin"           # "builtin" | "dms" | "system" | "manual"
name = "auto"                # "auto" follows light/dark mode; or "dark", "light",
                             # "catppuccin-mocha", "catppuccin-latte", "nord", or
                             # a palette file in ~/.config/ion/themes/<name>.toml
# palette = "/path/to/palette.toml"   # "manual" defaults to ~/.config/ion/palette.toml,
                                      # "dms" to ~/.config/ion/dms-palette.toml
pages = "match"              # what web pages see: "match" gives them the theme's
                             # light/dark (prefers-color-scheme), "system" the
                             # system's; "darken" is "match" plus darkening pages
                             # that have no dark style when the theme is dark

[theme.sites."example.com"]  # per site; also covers subdomains, most specific wins
# darken = false             # never darken this site (true: darken under a dark
                             # theme even when pages isn't "darken")
# css = "body { max-width: 50em; margin: auto }"   # added to the site's pages

[bangs]                      # your own, on top of Ion's built-in set
# gh = "https://github.com/search?q={}"   # adds or replaces !gh
# yt = ""                                 # removes the built-in !yt

[adblock]
enable = true
# Names: easylist, easyprivacy, ublock-filters (brings ublock-unbreak and
# ublock-quick-fixes along), ublock-privacy; or https:// URLs of other lists.
lists = ["easylist", "easyprivacy", "ublock-filters"]

[shortcuts]                  # command id = Qt key sequence
# palette = "Ctrl+K"
```

## Nix

```nix
# flake inputs: ion.url = "github:Mischa-dev/ion";
home-manager.users.me = {
  imports = [ ion.homeManagerModules.default ];
  programs.ion = {
    enable = true;
    theme.source = "dms";
    ui = { density = "compact"; cornerRadius = 8; tabs = "vertical"; };
    bangs.nix = "https://search.nixos.org/packages?query={}";
  };
};
```

Enum and number options are typed, so typos fail at `home-manager switch`.
Other keys pass through to the TOML unchanged. "Promote to Nix" in the app
(`Config.overridesAsNix()`) prints the overrides as a `programs.ion` snippet.

## For feature code

- Rust: `ion_config::global().config()` is the current `Config`;
  `ion_config::global().subscribe(|state| …)` hears every change (keep the
  returned `Subscription` alive). Callbacks run off the Qt thread; queue onto it
  with `CxxQtThread::queue`, as `bridge/config.rs` does.
- QML: the `Config` singleton has typed properties (`Config.homePage`,
  `Config.cornerRadius`, `Config.tabLayout`, …) that update live, plus
  `Config.value("dotted.key")` (bind on `Config.revision` to re-evaluate),
  `Config.set(key, value)` and `Config.reset(key)`, which return `""` or an
  error message.
- A new setting: add the field with a default in
  `crates/ion-config/src/schema.rs`, a typed option in `nix/hm-module.nix`, and
  (if QML needs it often) a property in `bridge/config.rs`.
