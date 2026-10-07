//! Ion's configuration: a typed schema over layered TOML files, with live
//! reload and a writable overrides layer for the settings UI.
//!
//! Two files live in the config directory (see [`Paths::from_env`]):
//!
//! - `config.toml`, the base. home-manager's `programs.ion` module generates
//!   it; without Nix it is edited by hand. Ion never writes it.
//! - `overrides.toml`, written by Ion's settings UI through [`Store::set`] and
//!   [`Store::reset`]. Its values win.
//!
//! Most code only needs the process-wide store:
//!
//! ```no_run
//! let config = ion_config::global().config();
//! println!("search with {}", config.search.engine);
//!
//! // Keep the subscription alive for as long as you want updates.
//! let _subscription = ion_config::global().subscribe(|state| {
//!     println!("corner radius is now {}", state.config.ui.corner_radius);
//! });
//! ```
//!
//! Subscribers run on the thread that reloaded (the file watcher, or the
//! caller of `set`); Qt objects hop to their own thread with
//! `CxxQtThread::queue` (see `ion-app/src/bridge/config.rs`).

mod layer;
pub mod nix;
mod schema;
mod store;

use std::sync::OnceLock;

pub use schema::{
    Adblock, Animations, Config, Density, General, Search, Site, TabLayout, Theme, ThemeSource, Ui,
};
pub use store::{Paths, State, Store, Subscription, Watcher};
pub use toml::Value;

/// The process-wide store, loaded from [`Paths::from_env`] and watched for
/// changes on first use. Falls back to defaults when there is no home
/// directory; a failure to watch only means no live reload.
pub fn global() -> &'static Store {
    static GLOBAL: OnceLock<Store> = OnceLock::new();
    GLOBAL.get_or_init(|| {
        let Some(paths) = Paths::from_env() else {
            return Store::in_memory();
        };
        let store = Store::open(paths);
        match store.watch() {
            // The watcher lives as long as the process, like the store.
            Ok(watcher) => std::mem::forget(watcher),
            Err(e) => eprintln!("ion: config live reload is off: {e}"),
        }
        store
    })
}
