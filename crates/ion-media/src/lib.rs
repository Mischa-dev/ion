//! Media in Ion: what pages may play on their own, and the scripts that
//! enforce it.
//!
//! - [`autoplay`]: the `[media] autoplay` policy for a URL, after `[sites]`
//!   overrides, and the page script behind `"block"`.
//! - [`pip`]: picture-in-picture, made by Ion (QtWebEngine has none): the
//!   scripts that make a video fill its tab, and the page's
//!   `requestPictureInPicture()` turned into a request to Ion.
//! - [`session`]: what each tab is playing (Media Session metadata, play
//!   state) and the actions Ion's and the system's media controls send.

pub mod autoplay;
pub mod pip;
pub mod session;

pub use autoplay::autoplay_for;
