//! Bookmarks for the Ion browser, and importing bookmarks and history from
//! other browsers.
//!
//! - [`store`]: the bookmark list, saved as `bookmarks.json` in Ion's data dir.
//! - [`import`]: reads Chrome-family and Firefox profiles.

pub mod import;
pub mod store;

pub use store::{Bookmark, Bookmarks};
