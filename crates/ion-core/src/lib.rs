//! Ion's core logic.
//!
//! This crate is deliberately free of Qt so it can be unit tested on its own
//! and reused outside the desktop shell. Everything a person sees lives in
//! `ion-app` (QML + the cxx-qt bridge); everything that decides *what* happens
//! lives here or in a sibling `ion-*` crate.

pub mod navigation;

/// Human-facing application name.
pub const APP_NAME: &str = "Ion";

/// Reverse-DNS style identifier, used for settings paths and the desktop file.
pub const APP_ID: &str = "dev.ion.Ion";

/// Crate version, shared by the whole workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
