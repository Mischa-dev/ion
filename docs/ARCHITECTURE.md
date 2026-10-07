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
crates/
  ion-core/               shared core: app constants, URL-bar input resolution
  ion-bangs/              !bangs and the command palette's ranking
  ion-app/                the binary (`ion`)
    build.rs              auto-discovers bridges, C++ shims and QML files
    src/main.rs           startup: QtWebEngine init, app, QML engine
    src/bridge/           one cxx-qt QObject per file, exposed to QML as `import Ion`
    cpp/                  small C++ shims for QtWebEngine APIs
    qml/Main.qml          window, tab model, shortcuts
    qml/Theme.qml         design tokens singleton (colors, radius, density, motion)
    qml/components/       UI pieces (TabStrip, NavigationBar, UrlBar, BrowserTab…)
packaging/                desktop file, later the macOS bundle bits
docs/
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

- Tabs live in a QML `ListModel` in `Main.qml`. The sessions work moves them
  into a Rust model so they can be saved, restored and driven by agents.
- `Theme.qml` holds static tokens. The theming work drives them from Rust
  (built-in themes, DMS palette file, system accent) without changing the
  components that read them.
- `ion_core::navigation::Omnibox` decides between address and search. Extra
  stages implement `InputStep` and run first; `ion_bangs::BangTable` is one.
  Config feeds user bangs through `BangTable::apply`.
- The Ctrl/Cmd+K palette ranks results in `ion_bangs::palette`; commands are
  listed in `ion_bangs::commands` and carried out in `CommandPalette.qml`.
- The browser profile is persistent (`storageName: "Default"`).
- Settings come from `ion-config` (layered TOML, live reload, `programs.ion`
  home-manager module); see [CONFIG.md](CONFIG.md).
