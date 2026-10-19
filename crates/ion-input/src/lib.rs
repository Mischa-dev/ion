//! Keyboard shortcuts and pointer gestures.
//!
//! - [`keymap`]: which keys run which command, after `[shortcuts]` remaps.
//! - [`swipe`]: two-finger touchpad swipes for back and forward.
//! - [`pinch`]: touchpad pinch to zoom the page.
//! - [`mouse`]: right-button mouse gestures.
//!
//! Qt-free; `ion-app` feeds it key strings and pointer events and
//! carries out what it decides (`bridge/keymap.rs`, `bridge/gestures.rs`).

pub mod keymap;
pub mod mouse;
pub mod pinch;
pub mod swipe;

pub use keymap::{Binding, Keymap};
