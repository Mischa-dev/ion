//! Per-site settings, user styles and userscripts.
//!
//! - [`rules`]: which `[sites]` entries from config apply to a URL, and what
//!   they add up to (JavaScript on or off, extra CSS).
//! - [`scripts`]: everything Ion injects into pages as WebEngine user scripts:
//!   the per-site CSS, plus `*.user.js` and `*.user.css` files from the
//!   `userscripts` folder in Ion's config directory.

pub mod rules;
pub mod scripts;

pub use rules::{javascript_enabled, matching};
pub use scripts::{RunAt, Script, World, all_scripts, load_dir, site_styles};
