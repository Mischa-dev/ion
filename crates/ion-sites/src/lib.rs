//! Per-site settings, user styles and userscripts.
//!
//! - [`rules`]: which `[sites]` entries from config apply to a URL, and what
//!   they add up to (JavaScript on or off).
//! - [`agent`]: the per-site user agent, for request headers and pages.
//! - [`scripts`]: everything Ion injects into pages as WebEngine user scripts:
//!   `*.user.js` and `*.user.css` files from the `userscripts` folder in Ion's
//!   config directory, Global Privacy Control and keyboard mode. Per-site CSS
//!   is `[theme.sites]`, in `ion-theme`.

pub mod agent;
pub mod rules;
pub mod scripts;

pub use rules::{javascript_enabled, matching};
pub use scripts::{RunAt, Script, World, all_scripts, load_dir};
