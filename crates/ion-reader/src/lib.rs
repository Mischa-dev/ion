//! Reader mode: pull the article out of a page and render it as a clean,
//! themed page.
//!
//! [`extract`] scores the page's blocks of prose (an Arc90/Readability-style
//! heuristic: long paragraphs with commas count, link-heavy blocks don't) and
//! keeps the best container, re-serialized through an allowlist of tags and
//! attributes so no script, style or tracking markup survives.
//! [`render`] wraps an [`Article`] in a standalone page coloured by the active
//! theme, with a Content-Security-Policy that blocks every script.

mod extract;
mod render;

pub use extract::{Article, extract};
pub use render::{Style, render};
