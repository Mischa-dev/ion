//! Platform integration that does not need Qt.
//!
//! - [`chromium`]: the Chromium switches Ion hands QtWebEngine at startup
//!   (hardware video decoding on Linux), merged with the user's own.
//! - [`instance`]: one Ion process per profile. A second launch (a link
//!   clicked in another app, `ion https://…` in a terminal) hands its URLs to
//!   the running process over a local socket and exits, because two processes
//!   cannot share one QtWebEngine profile.

pub mod chromium;
pub mod instance;
