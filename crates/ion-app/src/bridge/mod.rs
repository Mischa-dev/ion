//! cxx-qt bridges exposing the Rust core to QML.
//!
//! One file per QML-facing object. `build.rs` picks up every `*.rs` file in
//! this directory automatically; only the `mod` line below is shared.

pub mod adblock;
pub mod agent;
pub mod basics;
pub mod bookmarks;
pub mod config;
pub mod downloads;
pub mod extensions;
pub mod history;
pub mod load_error;
pub mod omnibox;
pub mod palette;
pub mod platform;
pub mod privacy;
pub mod reader;
pub mod safety;
pub mod screenshot;
pub mod sites;
pub mod tabs;
pub mod theme;
pub mod webengine;
pub mod zoom;
