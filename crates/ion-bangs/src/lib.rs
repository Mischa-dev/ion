//! Bangs and the command palette.
//!
//! - [`bang`]: the `!name query` table and the [`InputStep`] that plugs it into
//!   the URL bar's resolve pipeline.
//! - [`commands`]: the Ion commands the palette can run.
//! - [`palette`]: ranking tabs, commands and bangs for the Ctrl/Cmd+K palette.
//!
//! Qt-free; `ion-app` adapts it to QML in `bridge/palette.rs`.
//!
//! [`InputStep`]: ion_core::navigation::InputStep

pub mod bang;
pub mod commands;
mod fuzzy;
pub mod palette;

pub use bang::{Bang, BangTable};
pub use commands::{COMMANDS, Command};
pub use palette::{Action, Item, Kind, Palette, TabEntry};
