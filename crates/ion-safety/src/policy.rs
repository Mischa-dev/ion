//! Deciding: hard limits, stop and take-over, rules, then trust levels.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::capability::{Action, SiteCapability, Tier};
use crate::rule::{self, Effect, Lifetime, Rule, Source, What, Who};
use crate::site::{Origin, SitePattern};

/// The id of Ion's built-in agent.
pub const ION_AGENT: &str = "ion";

/// Schemes of pages agents may never read or act on: Ion's own pages,
/// the engine's internal pages and extension pages (password managers live
/// there).
pub const PROTECTED_SCHEMES: [&str; 7] = [
    "ion",
    "chrome",
    "chrome-extension",
    "chrome-untrusted",
    "devtools",
    "view-source",
    "qrc",
];

/// How much an agent may do without asking.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum TrustLevel {
    /// Reads and acts only on sites the person approved; asks for every new
    /// site and before committing, personal data or purchases.
    #[default]
    Ask,
    /// Acts freely (including commit) on its trusted sites; elsewhere as
    /// `Ask`. Still asks for personal data and purchases.
    TrustedSites,
    /// Acts anywhere without asking, within the hard limits.
    Full,
    /// Only its rules decide; anything they leave open asks.
    Custom,
}

impl TrustLevel {
    pub fn id(self) -> &'static str {
        match self {
            TrustLevel::Ask => "ask",
            TrustLevel::TrustedSites => "trustedSites",
            TrustLevel::Full => "full",
            TrustLevel::Custom => "custom",
        }
    }

    pub fn from_id(id: &str) -> Option<TrustLevel> {
        [
            TrustLevel::Ask,
            TrustLevel::TrustedSites,
            TrustLevel::Full,
            TrustLevel::Custom,
        ]
        .into_iter()
        .find(|t| t.id() == id)
    }
}

/// A rule from an agent's config section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigRule {
    pub what: What,
    pub site: SitePattern,
    pub effect: Effect,
}

/// How one agent is set up.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentProfile {
    /// Name shown in prompts; empty means derive it from the id.
    pub name: String,
    pub trust: TrustLevel,
    /// Sites a `TrustedSites` agent acts on freely.
    pub trusted_sites: Vec<SitePattern>,
    /// Connectors the agent may use without asking.
    pub connectors: Vec<String>,
    pub rules: Vec<ConfigRule>,
}

/// The name people see for an agent: its profile name, "Ion Agent" for the
/// built-in agent, or the id.
pub fn display_name(id: &str, profile: Option<&AgentProfile>) -> String {
    match profile {
        Some(p) if !p.name.trim().is_empty() => p.name.trim().to_owned(),
        _ if id == ION_AGENT => "Ion Agent".to_owned(),
        _ => id.to_owned(),
    }
}

/// An agent asking to do something.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRequest {
    pub agent: String,
    pub action: Action,
    /// The site the action lands on: the page's origin, or for `navigate` the
    /// destination's. `None` for actions not about a site.
    pub site: Option<Origin>,
    /// The tab acted on, if any.
    pub tab: Option<u64>,
}

/// A page asking to use something.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteRequest {
    pub origin: Origin,
    pub capability: SiteCapability,
    pub tab: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    Allow,
    Ask,
    Deny,
}

impl From<Effect> for Verdict {
    fn from(effect: Effect) -> Verdict {
        match effect {
            Effect::Allow => Verdict::Allow,
            Effect::Ask => Verdict::Ask,
            Effect::Deny => Verdict::Deny,
        }
    }
}

/// Limits no setting can lift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HardLimit {
    /// Passwords, payment details, cookies and tokens.
    Credentials,
    /// Ion's own pages, engine pages and extension pages.
    ProtectedPage,
    /// A site action without a site: a bug in the caller, refused.
    MissingSite,
    /// An action on a tab without the tab: refused, so taking a tab back
    /// can't be sidestepped.
    MissingTab,
}

/// Why a decision came out the way it did.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Reason {
    HardLimit {
        limit: HardLimit,
    },
    /// The person stopped all agents.
    Stopped,
    /// The person paused this agent.
    Paused,
    /// The person took this tab back from agents.
    TakenOver,
    /// The tab closed before the prompt was answered.
    TabClosed,
    /// A rule decided.
    Rule {
        source: Source,
        lifetime: Lifetime,
    },
    /// The agent's trust level decided.
    Trust {
        level: TrustLevel,
    },
    /// Pages without a host (local files, `data:`) always ask agents.
    LocalPage,
    /// Nothing decided; sites and unknown cases ask.
    Default,
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reason::HardLimit {
                limit: HardLimit::Credentials,
            } => f.write_str("Agents never see saved passwords, payment details or sign-in tokens"),
            Reason::HardLimit {
                limit: HardLimit::ProtectedPage,
            } => f.write_str("Agents can't use Ion's own pages, settings or extensions"),
            Reason::HardLimit {
                limit: HardLimit::MissingSite,
            } => f.write_str("The request didn't say which site it was for"),
            Reason::HardLimit {
                limit: HardLimit::MissingTab,
            } => f.write_str("The request didn't say which tab it was for"),
            Reason::Stopped => f.write_str("All agents are stopped"),
            Reason::Paused => f.write_str("This agent is paused"),
            Reason::TakenOver => f.write_str("You took this tab back"),
            Reason::TabClosed => f.write_str("The tab was closed"),
            Reason::Rule {
                source: Source::Config,
                ..
            } => f.write_str("A rule in your config"),
            Reason::Rule { lifetime, .. } => f.write_str(match lifetime {
                Lifetime::Once => "Your answer just now",
                Lifetime::Tab(_) => "Your answer for this tab",
                Lifetime::Session => "Your answer earlier this session",
                Lifetime::Forever => "A choice you saved",
            }),
            Reason::Trust { level } => write!(f, "The agent's trust level ({})", level.id()),
            Reason::LocalPage => f.write_str("Local pages always ask"),
            Reason::Default => f.write_str("Ion asks by default"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub verdict: Verdict,
    pub reason: Reason,
}

impl Decision {
    fn new(verdict: Verdict, reason: Reason) -> Decision {
        Decision { verdict, reason }
    }

    fn from_rule(rule: &Rule) -> Decision {
        Decision::new(
            rule.effect.into(),
            Reason::Rule {
                source: rule.source,
                lifetime: rule.lifetime,
            },
        )
    }
}

/// Everything decisions depend on, in memory. [`crate::Safety`] wraps it
/// with storage, prompts and the activity log.
#[derive(Debug, Clone, Default)]
pub struct Policy {
    profiles: BTreeMap<String, AgentProfile>,
    config_rules: Vec<Rule>,
    user_rules: Vec<Rule>,
    stopped: bool,
    paused: BTreeSet<String>,
    taken_over: BTreeSet<u64>,
}

impl Policy {
    pub fn new() -> Policy {
        Policy::default()
    }

    /// Replace the agent profiles (from config). Their rules become config
    /// rules; user rules are kept.
    pub fn set_profiles(&mut self, profiles: BTreeMap<String, AgentProfile>) {
        self.config_rules = profiles
            .iter()
            .flat_map(|(id, profile)| {
                profile.rules.iter().map(move |r| Rule {
                    who: Who::Agent(id.clone()),
                    what: r.what.clone(),
                    site: r.site.clone(),
                    effect: r.effect,
                    lifetime: Lifetime::Forever,
                    source: Source::Config,
                    created: 0,
                })
            })
            .collect();
        self.profiles = profiles;
    }

    pub fn profile(&self, agent: &str) -> Option<&AgentProfile> {
        self.profiles.get(agent)
    }

    pub fn display_name(&self, agent: &str) -> String {
        display_name(agent, self.profile(agent))
    }

    pub fn trust(&self, agent: &str) -> TrustLevel {
        self.profile(agent).map(|p| p.trust).unwrap_or_default()
    }

    pub fn user_rules(&self) -> &[Rule] {
        &self.user_rules
    }

    pub fn config_rules(&self) -> &[Rule] {
        &self.config_rules
    }

    /// Add a rule the person made. `Once` rules are never stored. An existing
    /// rule for the same who, what and site is replaced.
    pub fn add_user_rule(&mut self, rule: Rule) {
        if rule.lifetime == Lifetime::Once {
            return;
        }
        self.user_rules
            .retain(|r| !(r.who == rule.who && r.what == rule.what && r.site == rule.site));
        self.user_rules.push(rule);
    }

    /// Remove the user rules `keep` returns false for. Returns how many went.
    pub fn retain_user_rules(&mut self, mut keep: impl FnMut(&Rule) -> bool) -> usize {
        let before = self.user_rules.len();
        self.user_rules.retain(|r| keep(r));
        before - self.user_rules.len()
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    pub fn set_stopped(&mut self, stopped: bool) {
        self.stopped = stopped;
    }

    pub fn is_paused(&self, agent: &str) -> bool {
        self.paused.contains(agent)
    }

    pub fn set_paused(&mut self, agent: &str, paused: bool) {
        if paused {
            self.paused.insert(agent.to_owned());
        } else {
            self.paused.remove(agent);
        }
    }

    pub fn is_taken_over(&self, tab: u64) -> bool {
        self.taken_over.contains(&tab)
    }

    pub fn set_taken_over(&mut self, tab: u64, taken: bool) {
        if taken {
            self.taken_over.insert(tab);
        } else {
            self.taken_over.remove(&tab);
        }
    }

    /// Forget everything tied to a closed tab.
    pub fn tab_closed(&mut self, tab: u64) {
        self.taken_over.remove(&tab);
        self.user_rules.retain(|r| r.lifetime != Lifetime::Tab(tab));
    }

    /// Drop session and tab rules, as when Ion restarts.
    pub fn end_session(&mut self) {
        self.user_rules.retain(|r| r.lifetime == Lifetime::Forever);
        self.taken_over.clear();
        self.paused.clear();
        self.stopped = false;
    }

    fn rules(&self) -> impl Iterator<Item = &Rule> {
        self.config_rules.iter().chain(&self.user_rules)
    }

    /// May the page at `request.origin` use `request.capability`?
    pub fn decide_site(&self, request: &SiteRequest) -> Decision {
        let covering = self
            .rules()
            .filter(|r| r.covers_site(&request.origin, request.capability, request.tab));
        match rule::deciding(covering) {
            Some(rule) => Decision::from_rule(rule),
            None => Decision::new(Verdict::Ask, Reason::Default),
        }
    }

    /// May the agent do this?
    pub fn decide_agent(&self, request: &AgentRequest) -> Decision {
        use Verdict::{Allow, Ask, Deny};
        let deny_hard = |limit| Decision::new(Deny, Reason::HardLimit { limit });

        // 1. Hard limits.
        let tier = request.action.tier();
        if tier == Tier::Forbidden {
            return deny_hard(HardLimit::Credentials);
        }
        if request.action.needs_site() && request.site.is_none() {
            return deny_hard(HardLimit::MissingSite);
        }
        if request.action.needs_tab() && request.tab.is_none() {
            return deny_hard(HardLimit::MissingTab);
        }
        if request
            .site
            .as_ref()
            .is_some_and(|site| PROTECTED_SCHEMES.contains(&site.scheme()))
        {
            return deny_hard(HardLimit::ProtectedPage);
        }

        // 2. Stop, pause and take-over.
        if self.stopped {
            return Decision::new(Deny, Reason::Stopped);
        }
        if self.is_paused(&request.agent) {
            return Decision::new(Deny, Reason::Paused);
        }
        if request.tab.is_some_and(|tab| self.is_taken_over(tab)) {
            return Decision::new(Deny, Reason::TakenOver);
        }

        // 3. Rules.
        let covering = self.rules().filter(|r| {
            r.covers_agent(
                &request.agent,
                &request.action,
                request.site.as_ref(),
                request.tab,
            )
        });
        if let Some(rule) = rule::deciding(covering) {
            return Decision::from_rule(rule);
        }

        // 4. Trust level.
        let profile = self.profile(&request.agent);
        let level = profile.map(|p| p.trust).unwrap_or_default();
        let by_trust = |verdict| Decision::new(verdict, Reason::Trust { level });

        if let Action::UseConnector(name) = &request.action {
            let listed = profile.is_some_and(|p| p.connectors.iter().any(|c| c == name));
            return match level {
                TrustLevel::Full => by_trust(Allow),
                _ if listed => by_trust(Allow),
                _ => by_trust(Ask),
            };
        }

        let Some(site) = &request.site else {
            // Not about a site (listing tabs).
            return by_trust(if level == TrustLevel::Full {
                Allow
            } else {
                Ask
            });
        };
        if site.host().is_none() {
            return Decision::new(Ask, Reason::LocalPage);
        }
        match level {
            TrustLevel::Full => by_trust(Allow),
            TrustLevel::TrustedSites => {
                let trusted =
                    profile.is_some_and(|p| p.trusted_sites.iter().any(|s| s.matches(Some(site))));
                by_trust(if trusted && tier <= Tier::Commit {
                    Allow
                } else {
                    Ask
                })
            }
            TrustLevel::Ask | TrustLevel::Custom => by_trust(Ask),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(s: &str) -> Origin {
        Origin::parse(s).unwrap()
    }

    fn req(agent: &str, action: Action, site: Option<&str>) -> AgentRequest {
        AgentRequest {
            agent: agent.into(),
            action,
            site: site.map(origin),
            tab: Some(1),
        }
    }

    fn profile(trust: TrustLevel) -> AgentProfile {
        AgentProfile {
            trust,
            ..AgentProfile::default()
        }
    }

    fn policy_with(id: &str, p: AgentProfile) -> Policy {
        let mut policy = Policy::new();
        policy.set_profiles([(id.to_owned(), p)].into());
        policy
    }

    fn user_rule(agent: &str, what: What, site: &str, effect: Effect, lifetime: Lifetime) -> Rule {
        Rule {
            who: Who::Agent(agent.into()),
            what,
            site: SitePattern::parse(site).unwrap(),
            effect,
            lifetime,
            source: Source::User,
            created: 0,
        }
    }

    const GH: Option<&str> = Some("https://github.com/x");

    #[test]
    fn credentials_are_denied_at_full_trust() {
        let policy = policy_with("ion", profile(TrustLevel::Full));
        let d = policy.decide_agent(&req("ion", Action::ReadCredentials, GH));
        assert_eq!(d.verdict, Verdict::Deny);
        assert_eq!(
            d.reason,
            Reason::HardLimit {
                limit: HardLimit::Credentials
            }
        );
    }

    #[test]
    fn rules_cannot_lift_hard_limits() {
        let mut policy = policy_with("ion", profile(TrustLevel::Full));
        policy.add_user_rule(user_rule(
            "ion",
            What::Any,
            "*",
            Effect::Allow,
            Lifetime::Forever,
        ));
        for site in [
            "ion://settings",
            "chrome://gpu",
            "chrome-extension://abc/popup.html",
        ] {
            let d = policy.decide_agent(&req("ion", Action::ReadPage, Some(site)));
            assert_eq!(d.verdict, Verdict::Deny, "{site}");
            assert_eq!(
                d.reason,
                Reason::HardLimit {
                    limit: HardLimit::ProtectedPage
                }
            );
        }
        assert_eq!(
            policy
                .decide_agent(&req("ion", Action::ReadCredentials, GH))
                .verdict,
            Verdict::Deny
        );
    }

    #[test]
    fn site_actions_need_a_site() {
        let policy = policy_with("ion", profile(TrustLevel::Full));
        let d = policy.decide_agent(&req("ion", Action::Interact, None));
        assert_eq!(
            d.reason,
            Reason::HardLimit {
                limit: HardLimit::MissingSite
            }
        );
        assert_eq!(
            policy
                .decide_agent(&req("ion", Action::ListTabs, None))
                .verdict,
            Verdict::Allow
        );
    }

    #[test]
    fn tab_actions_need_a_tab() {
        let mut policy = policy_with("ion", profile(TrustLevel::Full));
        policy.set_taken_over(1, true);
        let mut no_tab = req("ion", Action::Interact, GH);
        no_tab.tab = None;
        let d = policy.decide_agent(&no_tab);
        assert_eq!(
            d.reason,
            Reason::HardLimit {
                limit: HardLimit::MissingTab
            }
        );
        let mut open = req("ion", Action::OpenTab, GH);
        open.tab = None;
        assert_eq!(policy.decide_agent(&open).verdict, Verdict::Allow);
        let mut list = req("ion", Action::ListTabs, None);
        list.tab = None;
        assert_eq!(policy.decide_agent(&list).verdict, Verdict::Allow);
    }

    #[test]
    fn stop_beats_everything_but_hard_limits() {
        let mut policy = policy_with("ion", profile(TrustLevel::Full));
        policy.add_user_rule(user_rule(
            "ion",
            What::Any,
            "*",
            Effect::Allow,
            Lifetime::Forever,
        ));
        policy.set_stopped(true);
        let d = policy.decide_agent(&req("ion", Action::ReadPage, GH));
        assert_eq!(d, Decision::new(Verdict::Deny, Reason::Stopped));
        policy.set_stopped(false);
        assert_eq!(
            policy
                .decide_agent(&req("ion", Action::ReadPage, GH))
                .verdict,
            Verdict::Allow
        );
    }

    #[test]
    fn pause_is_per_agent_and_take_over_per_tab() {
        let mut policy = Policy::new();
        policy.set_profiles(
            [
                ("ion".to_owned(), profile(TrustLevel::Full)),
                ("codex".to_owned(), profile(TrustLevel::Full)),
            ]
            .into(),
        );
        policy.set_paused("codex", true);
        assert_eq!(
            policy
                .decide_agent(&req("codex", Action::ReadPage, GH))
                .reason,
            Reason::Paused
        );
        assert_eq!(
            policy
                .decide_agent(&req("ion", Action::ReadPage, GH))
                .verdict,
            Verdict::Allow
        );

        policy.set_taken_over(1, true);
        assert_eq!(
            policy
                .decide_agent(&req("ion", Action::ReadPage, GH))
                .reason,
            Reason::TakenOver
        );
        let mut other_tab = req("ion", Action::ReadPage, GH);
        other_tab.tab = Some(2);
        assert_eq!(policy.decide_agent(&other_tab).verdict, Verdict::Allow);
        policy.tab_closed(1);
        assert!(!policy.is_taken_over(1));
    }

    #[test]
    fn ask_level_asks_on_new_sites_and_for_commit() {
        let mut policy = Policy::new(); // Unknown agents get `Ask`.
        for action in [
            Action::ReadPage,
            Action::Interact,
            Action::Submit,
            Action::Purchase,
        ] {
            assert_eq!(
                policy.decide_agent(&req("x", action, GH)).verdict,
                Verdict::Ask
            );
        }
        // Approving the site covers read and act, not commit.
        policy.add_user_rule(user_rule(
            "x",
            What::UpTo(Tier::Act),
            "github.com",
            Effect::Allow,
            Lifetime::Forever,
        ));
        assert_eq!(
            policy.decide_agent(&req("x", Action::ReadPage, GH)).verdict,
            Verdict::Allow
        );
        assert_eq!(
            policy.decide_agent(&req("x", Action::Navigate, GH)).verdict,
            Verdict::Allow
        );
        assert_eq!(
            policy.decide_agent(&req("x", Action::Submit, GH)).verdict,
            Verdict::Ask
        );
        assert_eq!(
            policy
                .decide_agent(&req("x", Action::ReadPage, Some("https://gitlab.com")))
                .verdict,
            Verdict::Ask
        );
    }

    #[test]
    fn trusted_sites_act_freely_but_ask_for_personal_and_purchase() {
        let policy = policy_with(
            "ion",
            AgentProfile {
                trust: TrustLevel::TrustedSites,
                trusted_sites: vec![SitePattern::parse("github.com").unwrap()],
                ..AgentProfile::default()
            },
        );
        let on = |a| {
            policy
                .decide_agent(&req("ion", a, Some("https://api.github.com")))
                .verdict
        };
        assert_eq!(on(Action::Interact), Verdict::Allow);
        assert_eq!(on(Action::Submit), Verdict::Allow);
        assert_eq!(on(Action::EnterPersonalData), Verdict::Ask);
        assert_eq!(on(Action::Purchase), Verdict::Ask);
        let off = policy.decide_agent(&req("ion", Action::ReadPage, Some("https://gitlab.com")));
        assert_eq!(off.verdict, Verdict::Ask);
    }

    #[test]
    fn full_trust_allows_everything_within_limits() {
        let policy = policy_with("ion", profile(TrustLevel::Full));
        for action in [
            Action::Submit,
            Action::Purchase,
            Action::UseConnector("gh".into()),
        ] {
            let site = if action.needs_site() { GH } else { None };
            assert_eq!(
                policy.decide_agent(&req("ion", action, site)).verdict,
                Verdict::Allow
            );
        }
    }

    #[test]
    fn local_pages_ask_even_at_full_trust_unless_a_rule_allows() {
        let mut policy = policy_with("ion", profile(TrustLevel::Full));
        let local = req("ion", Action::ReadPage, Some("file:///home/me/notes.html"));
        assert_eq!(
            policy.decide_agent(&local),
            Decision::new(Verdict::Ask, Reason::LocalPage)
        );
        policy.add_user_rule(user_rule(
            "ion",
            What::UpTo(Tier::Read),
            "file:///",
            Effect::Allow,
            Lifetime::Session,
        ));
        assert_eq!(policy.decide_agent(&local).verdict, Verdict::Allow);
    }

    #[test]
    fn custom_level_asks_where_rules_are_silent() {
        let policy = policy_with(
            "ion",
            AgentProfile {
                trust: TrustLevel::Custom,
                rules: vec![ConfigRule {
                    what: What::UpTo(Tier::Commit),
                    site: SitePattern::parse("example.com").unwrap(),
                    effect: Effect::Allow,
                }],
                ..AgentProfile::default()
            },
        );
        let ex = Some("https://example.com");
        assert_eq!(
            policy.decide_agent(&req("ion", Action::Submit, ex)).verdict,
            Verdict::Allow
        );
        let d = policy.decide_agent(&req("ion", Action::Submit, ex));
        assert_eq!(
            d.reason,
            Reason::Rule {
                source: Source::Config,
                lifetime: Lifetime::Forever
            }
        );
        assert_eq!(
            policy
                .decide_agent(&req("ion", Action::Purchase, ex))
                .verdict,
            Verdict::Ask
        );
        assert_eq!(
            policy
                .decide_agent(&req("ion", Action::ReadPage, GH))
                .verdict,
            Verdict::Ask
        );
    }

    #[test]
    fn config_deny_can_block_a_full_trust_agent_on_a_site() {
        let policy = policy_with(
            "ion",
            AgentProfile {
                trust: TrustLevel::Full,
                rules: vec![ConfigRule {
                    what: What::Any,
                    site: SitePattern::parse("bank.example").unwrap(),
                    effect: Effect::Deny,
                }],
                ..AgentProfile::default()
            },
        );
        let bank = Some("https://online.bank.example");
        assert_eq!(
            policy
                .decide_agent(&req("ion", Action::ReadPage, bank))
                .verdict,
            Verdict::Deny
        );
        assert_eq!(
            policy
                .decide_agent(&req("ion", Action::ReadPage, GH))
                .verdict,
            Verdict::Allow
        );
    }

    #[test]
    fn connectors_ask_unless_listed() {
        let policy = policy_with(
            "ion",
            AgentProfile {
                connectors: vec!["github".into()],
                ..AgentProfile::default()
            },
        );
        let use_ = |name: &str| {
            policy
                .decide_agent(&req("ion", Action::UseConnector(name.into()), None))
                .verdict
        };
        assert_eq!(use_("github"), Verdict::Allow);
        assert_eq!(use_("notes"), Verdict::Ask);
    }

    #[test]
    fn rules_for_one_agent_do_not_leak_to_another() {
        let mut policy = Policy::new();
        policy.add_user_rule(user_rule(
            "ion",
            What::Any,
            "*",
            Effect::Allow,
            Lifetime::Forever,
        ));
        assert_eq!(
            policy
                .decide_agent(&req("codex", Action::ReadPage, GH))
                .verdict,
            Verdict::Ask
        );
    }

    #[test]
    fn sites_ask_by_default_and_remember_answers() {
        let mut policy = Policy::new();
        let request = SiteRequest {
            origin: origin("https://meet.example"),
            capability: SiteCapability::Camera,
            tab: None,
        };
        assert_eq!(
            policy.decide_site(&request),
            Decision::new(Verdict::Ask, Reason::Default)
        );
        policy.add_user_rule(Rule {
            who: Who::Site,
            what: What::Site(SiteCapability::Camera),
            site: SitePattern::Origin(request.origin.clone()),
            effect: Effect::Deny,
            lifetime: Lifetime::Forever,
            source: Source::User,
            created: 0,
        });
        assert_eq!(policy.decide_site(&request).verdict, Verdict::Deny);
        let mic = SiteRequest {
            capability: SiteCapability::Microphone,
            ..request
        };
        assert_eq!(policy.decide_site(&mic).verdict, Verdict::Ask);
    }

    #[test]
    fn a_new_answer_replaces_the_old_one() {
        let mut policy = Policy::new();
        let rule = |effect| {
            user_rule(
                "ion",
                What::Action(Action::Submit),
                "github.com",
                effect,
                Lifetime::Forever,
            )
        };
        policy.add_user_rule(rule(Effect::Deny));
        policy.add_user_rule(rule(Effect::Allow));
        assert_eq!(policy.user_rules().len(), 1);
        assert_eq!(
            policy.decide_agent(&req("ion", Action::Submit, GH)).verdict,
            Verdict::Allow
        );
    }

    #[test]
    fn once_rules_are_not_kept_and_session_rules_end() {
        let mut policy = Policy::new();
        policy.add_user_rule(user_rule(
            "ion",
            What::Any,
            "*",
            Effect::Allow,
            Lifetime::Once,
        ));
        assert!(policy.user_rules().is_empty());
        policy.add_user_rule(user_rule(
            "ion",
            What::Any,
            "*",
            Effect::Allow,
            Lifetime::Session,
        ));
        policy.add_user_rule(user_rule(
            "ion",
            What::Any,
            "a.example",
            Effect::Allow,
            Lifetime::Forever,
        ));
        policy.add_user_rule(user_rule(
            "ion",
            What::Any,
            "b.example",
            Effect::Allow,
            Lifetime::Tab(3),
        ));
        policy.tab_closed(3);
        assert_eq!(policy.user_rules().len(), 2);
        policy.set_stopped(true);
        policy.end_session();
        assert_eq!(policy.user_rules().len(), 1);
        assert!(!policy.is_stopped());
    }

    #[test]
    fn display_names() {
        assert_eq!(display_name("ion", None), "Ion Agent");
        assert_eq!(display_name("claude-code", None), "claude-code");
        let named = AgentProfile {
            name: "Research".into(),
            ..AgentProfile::default()
        };
        assert_eq!(display_name("research", Some(&named)), "Research");
    }

    #[test]
    fn trust_level_ids() {
        for level in [
            TrustLevel::Ask,
            TrustLevel::TrustedSites,
            TrustLevel::Full,
            TrustLevel::Custom,
        ] {
            assert_eq!(TrustLevel::from_id(level.id()), Some(level));
        }
        assert_eq!(TrustLevel::from_id("yolo"), None);
    }
}
