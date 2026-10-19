//! Per-site settings, user styles and userscripts.
//!
//! - [`rules`]: which `[sites]` entries from config apply to a URL, and what
//!   they add up to (JavaScript on or off).
//! - [`agent`]: the per-site user agent, for request headers and pages.
//! - [`protocols`]: links that open other apps, and which sites may do so
//!   without asking.
//! - [`file_access`]: the question when a site wants files on this computer.
//! - [`scripts`]: everything Ion injects into pages as WebEngine user scripts:
//!   `*.user.js` and `*.user.css` files from the `userscripts` folder in Ion's
//!   config directory, Global Privacy Control and keyboard mode. Per-site CSS
//!   is `[theme.sites]`, in `ion-theme`.

pub mod agent;
pub mod file_access;
pub mod protocols;
pub mod rules;
pub mod scripts;

pub use rules::{javascript_enabled, matching};
pub use scripts::{RunAt, Script, World, all_scripts, load_dir};
