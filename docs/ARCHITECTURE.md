# Architecture

```
QML shell (crates/ion-app/qml)      what people see and touch
   │  properties, invokables, signals
cxx-qt bridge (crates/ion-app/src/bridge)
   │  plain Rust calls
Rust core (crates/ion-*)            all decisions, Qt-free, unit tested
QtWebEngine (from nixpkgs)          rendering, never patched by Ion
```

## Repository layout

```
flake.nix                 packages, dev shell, checks (CI runs `nix flake check`)
nix/
  package.nix             the `ion` package
  shell.nix               dev shell
  qt.nix                  the Qt modules Ion uses, merged for cxx-qt-build
  icons.nix               renders the app icon (hicolor PNGs, macOS .icns)
  packaging-check.nix     validates desktop file, AppStream data, Ion.app
crates/
  ion-core/               shared core: app constants, URL-bar input resolution
  ion-basics/             downloads, per-site zoom, permission prompts, page dialogs, find,
                          screenshots, context menu and new-tab page logic
  ion-platform/           Chromium switches, one-instance-per-profile hand-off
  ion-bangs/              !bangs and the command palette's ranking
  ion-session/            tabs and workspaces, saved and named sessions, history
  ion-bookmarks/          bookmarks; importing bookmarks and history from Chrome/Firefox
  ion-sites/              per-site settings ([sites] in config), user scripts and styles
  ion-reader/             reader mode: article extraction and the themed reader page
  ion-safety/             permissions and agent trust: decisions, prompts, activity log
  ion-app/                the binary (`ion`)
    build.rs              auto-discovers bridges, C++ shims and QML files
    src/main.rs           startup: QtWebEngine init, app, QML engine
    src/bridge/           one cxx-qt QObject per file, exposed to QML as `import Ion`
    cpp/                  small C++ shims for QtWebEngine and platform APIs
    icons/                the app icon (SVG source for every render)
    qml/Main.qml          window, web view stack, shortcuts
    qml/Theme.qml         design tokens singleton (colors, radius, density, motion)
    qml/components/       UI pieces (TabStrip, NavigationBar, UrlBar, BrowserTab…)
packaging/                Linux desktop file + AppStream, macOS Info.plist
docs/                     this file; platform.md for per-OS behaviour and checks
```

## Adding a feature module

Each v0.1 feature lives in its own crate and its own bridge file, so parallel
work rarely touches the same file:

1. `crates/ion-<feature>/`: the logic, Qt-free, with unit tests. Add it to
   `[workspace.dependencies]` in the root `Cargo.toml` and to
   `crates/ion-app/Cargo.toml`.
2. `crates/ion-app/src/bridge/<feature>.rs`: a `#[qml_element]` QObject that
   adapts the crate to QML, plus one `pub mod <feature>;` line in
   `bridge/mod.rs`. `build.rs` picks the file up by itself.
3. `crates/ion-app/qml/components/<Feature>*.qml` for its UI; files are added to
   the `Ion` QML module automatically. A file starting with `pragma Singleton` is
   registered as a singleton.
4. C++ only where QtWebEngine is much easier from C++ (for example a
   `QWebEngineUrlRequestInterceptor`): `crates/ion-app/cpp/<feature>.{h,cpp}`,
   exposed to Rust through an `unsafe extern "C++"` block in the bridge file.

Shared files with small, append-only edits: root `Cargo.toml`,
`crates/ion-app/Cargo.toml`, `bridge/mod.rs`, and the hookup in `Main.qml`.

## Where things stand

- Tabs live in the Rust `Tabs` singleton (`bridge/tabs.rs`, a list model over
  `ion_session::TabList`). Anything that opens, closes, moves or switches tabs
  calls its invokables; `Main.qml` keeps one web view per row and reports URL
  and title changes back. The open tabs are saved to
  `<data dir>/sessions/last.json` and restored at startup; restored background
  tabs load when first shown. Named sessions live next to it in `named/`.
- Every tab belongs to a workspace (`Tabs.workspace` is the current tab's).
  The strips filter the model to the current workspace with a
  `SortFilterProxyModel`; delegates use the `row` role, not their list index,
  when they talk to `Tabs`. Workspaces are saved with the session.
- `History` (`bridge/history.rs`) records finished page loads in
  `<data dir>/history.json`; `History.search(query, limit)` returns JSON for the
  command palette. The data dir is `$ION_DATA_DIR`, else
  `~/.local/share/ion` (XDG) or `~/Library/Application Support/Ion`.
- `Theme.qml`'s colors come from `ThemeEngine` (`crates/ion-theme`): built-in
  themes, DMS or manual palette files (live reloaded) and the system accent.
  `Theme.qml` binds them to the `[theme]` config section; see
  `crates/ion-theme/README.md`. Sizes and motion tokens are still static.
- Web pages follow the theme's light/dark (`theme.pages`), with per-site
  darkening and CSS (`[theme.sites]`) and page controls in the palette's colors
  (`theme.pageControls`); `BrowserTab` re-applies its settings when any of these
  changes. Palette-tinted dark mode isn't possible: Chromium hard-codes the
  darkened background.
- `ion_core::navigation::Omnibox` decides between address and search. Extra
  stages implement `InputStep` and run first; `ion_bangs::BangTable` is one.
  Config feeds user bangs through `BangTable::apply`.
- The Ctrl/Cmd+K palette ranks tabs, history, sessions, commands and bangs in
  `ion_bangs::palette`; commands are
  listed in `ion_bangs::commands` and carried out in `CommandPalette.qml`.
  `Palette::suggest` is the URL bar's narrower version (typed entry first,
  then tabs, history and bang completions), and `ion_bangs::autofill` fills
  in addresses inline; both lists draw their rows with
  `CommandPaletteRow.qml`.
- The browser profile is persistent (`storageName: "Default"`).
- Per-tab UI (find bar, permission prompt, page dialogs, context menu, new-tab page) is
  declared inside `BrowserTab.qml`; window-wide pieces (downloads panel,
  fullscreen) sit next to the view stack in `Main.qml`. Zoom levels are saved
  per site to `zoom.txt` in Ion's data directory.
- Settings come from `ion-config` (layered TOML, live reload, `programs.ion`
  home-manager module); see [CONFIG.md](CONFIG.md).
