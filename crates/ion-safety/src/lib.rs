//! Ion's permission and safety model: one place that answers "may this site
//! or agent do that here?", and one log of the answers.
//!
//! - [`policy::Policy::decide_site`] and [`policy::Policy::decide_agent`]
//!   return allow, deny or ask, with a reason. Order: hard limits, stop and
//!   take-over, rules (most specific first), the agent's trust level.
//! - [`prompt::Prompt`]: the sentence and choices shown when the answer is
//!   ask, and the rule each answer becomes.
//! - [`audit`]: the append-only activity log, one JSON Lines file per day.
//! - [`Safety`] ties them together with remembered rules on disk.
//!
//! The design, with the reasoning behind each default, is in
//! `docs/SAFETY.md`. Nothing here knows about Qt.

pub mod audit;
pub mod capability;
pub mod policy;
pub mod prompt;
pub mod rule;
mod safety;
pub mod site;
pub mod store;

pub use capability::{Action, SiteCapability, Tier};
pub use policy::{
    AgentProfile, AgentRequest, ConfigRule, Decision, Policy, Reason, SiteRequest, TrustLevel,
    Verdict,
};
pub use prompt::{Choice, Prompt};
pub use rule::{Effect, Lifetime, Rule, Source, What, Who};
pub use safety::{ActionRecord, AnswerError, Outcome, Safety};
pub use site::{Origin, SitePattern};

use std::sync::{Mutex, MutexGuard, OnceLock};

static GLOBAL: OnceLock<Mutex<Safety>> = OnceLock::new();

/// Make `safety` the process-wide instance. Call once at startup, before
/// anything uses [`global`]; later calls are ignored and return false.
pub fn install_global(safety: Safety) -> bool {
    GLOBAL.set(Mutex::new(safety)).is_ok()
}

/// The process-wide instance: the one installed at startup, or an in-memory
/// one (nothing saved or logged) if none was.
pub fn global() -> MutexGuard<'static, Safety> {
    GLOBAL
        .get_or_init(|| Mutex::new(Safety::in_memory()))
        .lock()
        // A panic while holding the lock leaves the policy as it was; keep
        // answering rather than taking the browser down with it.
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
