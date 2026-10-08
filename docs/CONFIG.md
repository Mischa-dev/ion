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
# Unpacked Chrome extensions to load, besides those in Ion's extensions folder
extensions = []              # e.g. ["/home/me/src/my-extension"]

[general]
homePage = "https://duckduckgo.com/"
restoreSession = true
suspendTabsAfter = 30        # minutes before an unseen background tab is unloaded; 0 = never

[search]
engine = "DuckDuckGo"
template = "https://duckduckgo.com/?q={}"

[ui]
density = "comfortable"      # "comfortable" | "compact"
cornerRadius = 8
tabs = "horizontal"          # "horizontal" | "vertical"
collapseSidebar = false      # vertical tabs: show only favicons until hovered
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
pageControls = true          # scrollbars, checkboxes and text selection on web
                             # pages use the theme's colors, unless the page
                             # styles them itself

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

[downloads]
# directory = "~/Downloads"  # empty: the system's download folder
[downloads.folders]          # per type: document, image, audio, video, archive, other
# image = "~/Pictures/Downloads"
# video = "~/Videos"

[newTab]
# mostVisited = true         # fill tiles with the sites visited most
# tiles = 8                  # how many tiles at most
# shortcuts = [              # pinned tiles, shown first
#   { title = "Mail", url = "https://mail.example/" },
# ]

[shortcuts]                  # command id = Qt key sequence
# palette = "Ctrl+K"
# askAgent = "Ctrl+E"        # the address bar, ready with "@ion " (needs ai.enable)

[keyboard]
vim = false                  # j/k scroll, gg/G, d/u, H/L back/forward,
                             # f/F link hints (F opens in a new tab)

[privacy]
globalPrivacyControl = true  # Sec-GPC header and navigator.globalPrivacyControl
blockThirdPartyCookies = true

[ai]                         # Ion Agent: @ion or !ai in the address bar
enable = false               # off until you turn it on
provider = "openai"          # any OpenAI-compatible chat completions API
model = "gpt-5-mini"
baseUrl = "https://api.openai.com/v1"   # e.g. http://127.0.0.1:11434/v1 for Ollama
apiKeyEnv = "OPENAI_API_KEY" # the key is read from this variable, never from config

[agents.claude]              # your own agent, asked with @claude (needs ai.enable)
name = "Claude"
model = "claude-sonnet-5-5"  # a model makes an agent askable; without one,
baseUrl = "https://api.anthropic.com/v1/"  # [agents.<id>] only sets its trust
apiKeyEnv = "ANTHROPIC_API_KEY"  # unset uses ai.apiKeyEnv; baseUrl "" uses ai.baseUrl
instructions = "Be brief. Cite the page."  # added to its system prompt
trust = "ask"                # see docs/SAFETY.md

[sites."example.com"]        # per-site settings; a host covers its subdomains,
javascript = true            # "*" covers every site, the most specific wins
                             # (CSS for a site goes in [theme.sites] above)
```

## User scripts and styles

Files in `~/.config/ion/userscripts/` are injected into pages:

- `*.user.js`: Greasemonkey-style userscripts. The `// ==UserScript==` header's
  `@match`, `@include`, `@exclude` and `@run-at` (`document-start`,
  `document-end`, `document-idle`) are honored. Scripts run in an isolated
  world that shares only the DOM with the page; `// @inject-into page` runs one
  in the page's own world instead. GM_* APIs are not provided.
- `*.user.css`: style sheets for every page, or only for the `@match` patterns
  listed in a leading `/* ==UserStyle== … ==/UserStyle== */` comment.

Run "Reload user scripts and styles" from the palette after editing them; "Open
user scripts folder" creates and opens the folder. Config changes apply on the
next page load. User scripts still run on sites with `javascript = false`.

## Extensions

Ion loads unpacked Chrome extensions (folders with a `manifest.json`) from its
extensions folder (`~/.local/share/ion/extensions/` on Linux,
`~/Library/Application Support/Ion/extensions/` on macOS) and from the
`extensions` list, at startup. QtWebEngine runs Manifest V3 only; older
extensions are listed as skipped, with the reason, in the Extensions dialog
(palette: "Extensions"). The dialog switches each one on or off until Ion
restarts and opens an extension's popup in a tab. With the Nix module,
`programs.ion.extensions` takes paths or packages that build an unpacked
extension.

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
