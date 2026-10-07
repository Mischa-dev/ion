//! cxx-qt bridges exposing the Rust core to QML.
//!
//! One file per QML-facing object. `build.rs` picks up every `*.rs` file in
//! this directory automatically; only the `mod` line below is shared.

pub mod basics;
pub mod config;
pub mod downloads;
pub mod history;
pub mod omnibox;
pub mod palette;
pub mod platform;
pub mod tabs;
pub mod theme;
pub mod webengine;
pub mod zoom;
