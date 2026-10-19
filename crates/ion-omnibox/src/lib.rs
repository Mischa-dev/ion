//! What makes Ion's address bar smart, Qt-free:
//!
//! - [`answer`]: inline answers for sums ([`calc`]) and unit conversions
//!   ([`units`]), shown as the first suggestion.
//! - [`inputs`]: what people picked after typing what, so `gm` learns to mean
//!   the mail page you always choose.
//! - [`remote`]: reading search engines' suggestion responses.
//! - [`suggest`]: ranking open tabs, bookmarks, history, quick actions and
//!   search suggestions for the dropdown under the address bar.
//!
//! The search engines themselves are in `ion_core::engines`; `ion-app`
//! adapts all of this to QML in `bridge/address_bar.rs`.

pub mod answer;
pub mod calc;
pub mod inputs;
pub mod remote;
pub mod suggest;
pub mod units;

pub use answer::Answer;
pub use inputs::InputHistory;
pub use suggest::{Page, Sources, suggest};
