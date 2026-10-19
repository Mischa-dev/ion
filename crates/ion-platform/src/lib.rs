//! Platform integration that does not need Qt.
//!
//! - [`chromium`]: the Chromium switches Ion hands QtWebEngine at startup
//!   (hardware video decoding and PipeWire screen capture on Linux, the
//!   Widevine CDM), merged with the user's own.
//! - [`desktop`]: the Qt platform theme that gives Linux portal file pickers.
//! - [`instance`]: one Ion process per profile. A second launch (a link
//!   clicked in another app, `ion https://…` in a terminal) hands its URLs to
//!   the running process over a local socket and exits, because two processes
//!   cannot share one QtWebEngine profile.
//! - [`extensions`]: which Chrome (Manifest V3) extensions to load.

pub mod chromium;
pub mod desktop;
pub mod extensions;
pub mod instance;
