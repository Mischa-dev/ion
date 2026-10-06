# Ion: notes for contributors and coding agents

Read `docs/ARCHITECTURE.md` first; it says which directory each feature owns.

## Commands (run inside `nix develop`)

- Build and run: `cargo run -p ion-app -- [urls…]`
- Unit tests: `cargo test --workspace`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Format: `cargo fmt --all` and `nixfmt flake.nix nix/*.nix`
- Everything CI runs: `nix flake check -L`

Outside the dev shell the build fails: cxx-qt needs `QMAKE` pointing at the
merged Qt prefix the shell sets up.

## Conventions

- Logic goes in a Qt-free `crates/ion-*` crate with unit tests. `ion-app` only
  adapts it to QML.
- One QML-facing object per file in `crates/ion-app/src/bridge/`; `build.rs`
  picks new bridge files, `cpp/*.cpp` shims and `qml/**/*.qml` up automatically.
- QML colors, sizes and durations come from `Theme` tokens, never literals.
- Shortcuts use `StandardKey` or `Ctrl+…` (Qt maps Ctrl to Cmd on macOS).
- C++ only for small shims where a QtWebEngine API is much easier from C++.
