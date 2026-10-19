//! Keyboard shortcuts and pointer gestures.
//!
//! - [`keymap`]: which keys run which command, after `[shortcuts]` remaps.
//!
//! Qt-free; `ion-app` feeds it key strings and carries out what it
//! decides (`bridge/keymap.rs`).

pub mod keymap;

pub use keymap::{Binding, Keymap};
