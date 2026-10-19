//! The side panel: web apps pinned beside the page, and notes.
//!
//! - [`web`]: the web panels (chat, music, mail…) and the panel's width.
//! - [`notes`]: quick notes and clippings from pages.
//!
//! Qt-free and file-free: `ion-app` loads and saves the JSON
//! (`bridge/panels.rs`).

pub mod notes;
pub mod web;

pub use notes::{Note, Notes};
pub use web::{Panels, WebPanel};
