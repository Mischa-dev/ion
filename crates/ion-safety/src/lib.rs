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
