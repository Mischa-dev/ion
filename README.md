# Ion

Ion is an experimental, fast and themeable desktop browser for Linux and
macOS. It combines Qt 6/QML and QtWebEngine (Chromium) with a Rust codebase
connected through [cxx-qt](https://github.com/KDAB/cxx-qt). Browser decisions
live in small, Qt-free Rust crates; the application crate is only the desktop
shell and its adapters.

> **Project status:** early foundation. Ion is useful for development and
> evaluation, but is not yet intended to replace a security-hardened browser.

Today Ion includes tabs and workspaces, session and history persistence,
bookmarks and browser import, `!bang` search, a command palette, ad blocking,
reader mode, print preview and a PDF viewer, picture-in-picture, system media
controls (MPRIS), opt-in page translation, per-site settings, userscripts,
themes, and a permission-gated browser agent.

## Quick start

The supported path uses [Nix with flakes enabled](https://nixos.org/download/).
It supplies the exact Rust toolchain, Qt 6 modules, native build tools, and
runtime plugin paths required by cxx-qt.

```sh
git clone https://github.com/Mischa-dev/ion.git
cd ion
nix run .
```

Supported flake systems are `x86_64-linux`, `aarch64-linux`, and
`aarch64-darwin`. The application needs a graphical session; builds and unit
tests do not.

### Development setup

```sh
nix develop                     # or: direnv allow
cargo run -p ion-app -- https://example.com
```

The dev shell sets `QMAKE`, compiler flags, Qt plugin paths, and the
`QtWebEngineProcess` path. A plain host Rust installation is therefore not a
complete build environment. Rust-only crates may still be edited and tested
individually outside Nix when their dependencies are already available.

Useful checks, matching CI:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
nixfmt --check flake.nix nix/*.nix
nix flake check -L              # build, tests, lints, and packaging checks
```

## Usage

Enter an address or search in the location bar. Type `!name query` (or
`query !name`) to search a site directly—for example `!gh cxx-qt`, `!nix
ripgrep`, or `!yt`. A bang by itself opens that site. Unknown bangs fall back
to the configured search engine. The built-in set lives in
`crates/ion-bangs/src/bang.rs`; add, replace or remove bangs in the `[bangs]`
config table ([docs/CONFIG.md](docs/CONFIG.md)).

The command palette (`Ctrl/Cmd+K`) searches tabs, history, saved sessions,
commands, settings, and bangs. Prefix a query with `!` to browse bangs or `>`
to browse commands and settings. Examples include `nord`, `vertical tabs`,
`search with kagi`, and `save session work`.

Location-bar suggestions show the action Enter will perform, matching open
tabs, history, and bang completions. Use Up/Down to select, Tab to complete a
bang, and Alt+Enter to open a result in a new tab. Addresses you have open or
visited fill in as you type ("git" → "github.com/"; keep typing or press
Backspace to drop it), and typing a bang's name without the `!` ("gh") offers
that site's search: press Tab to switch to it.

### Keyboard shortcuts

| Action | Shortcut |
| --- | --- |
| New / close tab | Ctrl/Cmd+T / Ctrl/Cmd+W |
| Reopen closed tab | Ctrl/Cmd+Shift+T |
| Search tabs | Ctrl/Cmd+Shift+A |
| Split view on / off | Ctrl/Cmd+Alt+S |
| Focus location bar | Ctrl/Cmd+L, Alt+D, F6 |
| Next / previous tab | Ctrl+Tab / Ctrl+Shift+Tab |
| Reload | Ctrl/Cmd+R, F5 |
| Back / forward | Alt+Left / Alt+Right (Cmd+[ / Cmd+] on macOS) |
| Command palette | Ctrl/Cmd+K |

Configuration is layered TOML with live reload. See
[the configuration guide](docs/CONFIG.md) for file locations, every setting,
userscripts, extensions, and the Home Manager module. Theme authors should
start with [the theme guide](crates/ion-theme/README.md). The permission and
agent trust guarantees are documented in [the safety model](docs/SAFETY.md).

## Architecture

```text
QML UI (crates/ion-app/qml)
        │ properties, invokables, signals
cxx-qt bridge (crates/ion-app/src/bridge)
        │ ordinary Rust calls
Qt-free Rust crates (crates/ion-*)
        │ engine and platform adapters
QtWebEngine / Chromium
```

The boundary is deliberate: UI-independent decisions remain quick to compile
and unit test, while QML owns presentation and C++ is limited to small Qt APIs
that are awkward to expose directly. `crates/ion-app/build.rs` discovers bridge
modules, QML files, and C++ shims automatically. See
[the architecture guide](docs/ARCHITECTURE.md) for data flow and instructions
for adding a feature.

## Repository layout

| Path | Purpose |
| --- | --- |
| `crates/ion-app` | `ion` binary, QML shell, cxx-qt bridges, and C++ Qt shims |
| `crates/ion-core` | Shared constants, navigation, and load-error decisions |
| `crates/ion-basics` | Downloads, zoom, dialogs, permissions, find, and new-tab logic |
| `crates/ion-bangs` | Bang expansion, suggestions, and command-palette ranking |
| `crates/ion-config` | Typed layered configuration, overrides, and live reload |
| `crates/ion-session` | Tabs, workspaces, saved sessions, and browsing history |
| `crates/ion-bookmarks` | Bookmark storage and Chrome/Firefox import |
| `crates/ion-adblock` | Filter-list storage, compilation, and request decisions |
| `crates/ion-sites` | Per-site rules, user agents, scripts, and styles |
| `crates/ion-theme` | Palettes, theme sources, page theming, and file watching |
| `crates/ion-reader` | Safe article extraction and reader-page rendering |
| `crates/ion-media` | Autoplay policy, picture-in-picture, and system media controls |
| `crates/ion-print` | Print preview page ranges, paper sizes, and saved-PDF names |
| `crates/ion-translate` | Opt-in page translation through Ion Agent's model |
| `crates/ion-safety` | Site permissions, agent policy, prompts, and audit trail |
| `crates/ion-agent` | Model-independent agent task state machine and browser tools |
| `crates/ion-platform` | Chromium flags, extensions, and single-instance handoff |
| `docs` | Architecture, configuration, safety, and platform guides |
| `nix` / `flake.nix` | Reproducible package, dev shell, Home Manager module, and checks |
| `packaging` | Linux desktop metadata and macOS application-bundle metadata |

## Contributing

Keep reusable behavior in a Qt-free crate and cover it with unit tests. The app
crate should adapt that behavior to QML rather than reimplement it. Before a
change, read [`CLAUDE.md`](CLAUDE.md) for repository conventions and run
`nix flake check -L`; platform UI behavior that CI cannot exercise is covered
by the manual checklists in [the platform guide](docs/platform.md).

When reporting a bug, include the OS/display server, the command used to start
Ion, and relevant terminal output. For rendering or media issues, also include
the `chrome://gpu` summary.
