//! Ion's theming: palettes, built-in themes and live theme sources.
//!
//! Qt-free. The `ThemeEngine` bridge in `ion-app` turns the active [`Palette`]
//! into the `Theme` QML singleton's color tokens and re-resolves it when the
//! source, the system appearance or a watched palette file changes.
//!
//! - [`palette`]: the token set and the TOML palette file format.
//! - [`builtin`]: themes compiled into Ion.
//! - [`pages`]: how web pages follow the theme.
//! - [`sites`]: per-site darkening and CSS.
//! - [`source`]: built-in, DMS, system and manual sources.
//! - [`watch`]: file watching for live reload.

pub mod builtin;
pub mod color;
pub mod pages;
pub mod palette;
pub mod sites;
pub mod source;
pub mod watch;

pub use color::Color;
pub use pages::PageTheming;
pub use palette::{Palette, Scheme};
pub use sites::SiteTheme;
pub use source::{Dirs, SystemAppearance, ThemeSource};
pub use watch::FileWatcher;

/// A fresh, empty directory for a test that touches the filesystem.
#[cfg(test)]
pub(crate) fn test_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ion-theme-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
