# Ion

A fast, themeable desktop browser for Linux and macOS: a Qt 6 / QML shell over
QtWebEngine (Chromium), with all logic in Rust via [cxx-qt](https://github.com/KDAB/cxx-qt).

Status: early foundation. It opens a window with tabs, a URL bar that
understands addresses and searches, and back / forward / reload.

## Run it

```sh
nix run github:Mischa-dev/ion            # once the repo is public
nix run .                                # from a checkout
```

## Develop

```sh
nix develop                  # or `direnv allow`
cargo run -p ion-app         # build and launch
cargo test --workspace       # unit tests
nix flake check -L           # everything CI runs
```

Supported systems: `x86_64-linux`, `aarch64-linux`, `aarch64-darwin`.

## Keyboard

| Action | Shortcut |
| --- | --- |
| New tab | Ctrl/Cmd+T |
| Close tab | Ctrl/Cmd+W |
| Focus URL bar | Ctrl/Cmd+L, Alt+D, F6 |
| Next / previous tab | Ctrl+Tab / Ctrl+Shift+Tab |
| Reload | Ctrl/Cmd+R, F5 |
| Back / forward | Alt+Left / Alt+Right (Cmd+[ / Cmd+] on macOS) |
| Command palette | Ctrl/Cmd+K |

## Bangs

Type `!name query` (or `query !name`) in the URL bar or the palette to search
a site directly, e.g. `!gh cxx-qt`, `!nix ripgrep`, `!yt`. A bang on its own
opens the site. The built-in set lives in `crates/ion-bangs/src/bang.rs`;
add, replace or remove bangs in the `[bangs]` config table (docs/CONFIG.md);
unknown bangs go to the search engine. The palette also changes settings in one
step ("nord", "vertical tabs", "search with kagi"). In the palette, `!` lists
bangs and `>` lists commands and settings.

## Layout

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
