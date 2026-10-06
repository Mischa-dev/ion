//! Ad and tracker blocking for Ion.
//!
//! Qt-free on purpose: `ion-app` asks [`Shield::decide`] about every network
//! request QtWebEngine makes (through a `QWebEngineUrlRequestInterceptor`) and
//! shows the counters and per-site switch this crate keeps.
//!
//! - [`lists`]: which filter lists Ion uses (EasyList, EasyPrivacy, uBlock Origin).
//! - [`store`]: the on-disk cache of downloaded lists and the compiled engine.
//! - [`update`]: refreshing stale lists and building a [`Blocker`] from them.
//! - [`Shield`]: the per-request decision, blocked counters and site switches.

pub mod blocker;
pub mod lists;
pub mod request;
pub mod shield;
pub mod sites;
pub mod store;
pub mod update;

pub use blocker::Blocker;
pub use lists::FilterList;
pub use request::{RequestInfo, ResourceType};
pub use shield::Shield;
pub use sites::SiteSettings;
pub use store::Store;
