//! Media in Ion: what pages may play on their own, and the scripts that
//! enforce it.
//!
//! - [`autoplay`]: the `[media] autoplay` policy for a URL, after `[sites]`
//!   overrides, and the page script behind `"block"`.

pub mod autoplay;

pub use autoplay::autoplay_for;
