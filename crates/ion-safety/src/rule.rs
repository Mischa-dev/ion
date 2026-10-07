//! Rules: remembered answers to "may this site or agent do that here?".

use std::cmp::Reverse;

use serde::{Deserialize, Serialize};

use crate::capability::{Action, SiteCapability, Tier};
use crate::site::{Origin, SitePattern};

/// Who a rule is about.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Who {
    /// The site matched by the rule's site pattern, asking for a site
    /// capability itself.
    Site,
    /// The agent with this id.
    Agent(String),
}

/// What a rule covers.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum What {
    /// One site capability (rules for sites).
    Site(SiteCapability),
    /// One agent action.
    Action(Action),
    /// Every action of this tier and the site tiers below it, so `tier:act`
    /// covers reading and acting. Connector and forbidden tiers cover only
    /// themselves.
    UpTo(Tier),
    /// Everything.
    Any,
}

impl What {
    /// Parse a config action pattern: an action id, `tier:<tier>`, or `*`.
    pub fn parse(text: &str) -> Option<What> {
        let text = text.trim();
        if text == "*" {
            return Some(What::Any);
        }
        if let Some(tier) = text.strip_prefix("tier:") {
            return Tier::from_id(tier).map(What::UpTo);
        }
        Action::from_id(text).map(What::Action)
    }

    fn covers_action(&self, action: &Action) -> bool {
        match self {
            What::Any => true,
            What::Action(a) => a == action,
            What::UpTo(tier) => {
                let t = action.tier();
                if tier.is_site_tier() {
                    t.is_site_tier() && t <= *tier
                } else {
                    t == *tier
                }
            }
            What::Site(_) => false,
        }
    }

    fn covers_site_capability(&self, capability: SiteCapability) -> bool {
        match self {
            What::Site(c) => *c == capability,
            What::Any => true,
            _ => false,
        }
    }

    fn specificity(&self) -> u32 {
        match self {
            What::Site(_) | What::Action(_) => 2,
            What::UpTo(_) => 1,
            What::Any => 0,
        }
    }
}

/// What a matching rule decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Effect {
    Allow,
    Ask,
    Deny,
}

/// How long a rule lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Lifetime {
    /// Answers one request and is never stored.
    Once,
    /// Until the tab with this id closes.
    Tab(u64),
    /// Until Ion quits.
    Session,
    /// Saved to disk.
    Forever,
}

/// Where a rule came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    /// The person, answering a prompt or in settings.
    User,
    /// Ion's config files (TOML or Nix). Read-only in the UI.
    Config,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub who: Who,
    pub what: What,
    pub site: SitePattern,
    pub effect: Effect,
    pub lifetime: Lifetime,
    pub source: Source,
    /// Unix seconds when the rule was made; 0 for config rules.
    #[serde(default)]
    pub created: u64,
}

impl Rule {
    /// Whether this rule covers `agent` doing `action` on `site` in `tab`.
    pub fn covers_agent(
        &self,
        agent: &str,
        action: &Action,
        site: Option<&Origin>,
        tab: Option<u64>,
    ) -> bool {
        matches!(&self.who, Who::Agent(id) if id == agent)
            && self.what.covers_action(action)
            && self.site.matches(site)
            && self.covers_tab(tab)
    }

    /// Whether this rule covers `origin` using `capability`.
    pub fn covers_site(
        &self,
        origin: &Origin,
        capability: SiteCapability,
        tab: Option<u64>,
    ) -> bool {
        self.who == Who::Site
            && self.what.covers_site_capability(capability)
            && self.site.matches(Some(origin))
            && self.covers_tab(tab)
    }

    fn covers_tab(&self, tab: Option<u64>) -> bool {
        match self.lifetime {
            Lifetime::Tab(id) => tab == Some(id),
            _ => true,
        }
    }

    /// Sort key: more specific rules first; at equal specificity deny before
    /// ask before allow.
    fn precedence(&self) -> (Reverse<u32>, Reverse<u32>, Reverse<Effect>) {
        (
            Reverse(self.what.specificity()),
            Reverse(self.site.specificity()),
            Reverse(self.effect),
        )
    }
}

/// The rule that decides, from those that cover a request: the most specific,
/// with deny beating ask beating allow on a tie.
pub fn deciding<'a>(covering: impl IntoIterator<Item = &'a Rule>) -> Option<&'a Rule> {
    covering.into_iter().min_by_key(|r| r.precedence())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent_rule(what: &str, site: &str, effect: Effect) -> Rule {
        Rule {
            who: Who::Agent("ion".into()),
            what: What::parse(what).unwrap(),
            site: SitePattern::parse(site).unwrap(),
            effect,
            lifetime: Lifetime::Forever,
            source: Source::User,
            created: 0,
        }
    }

    fn origin(s: &str) -> Origin {
        Origin::parse(s).unwrap()
    }

    #[test]
    fn what_parses_config_patterns() {
        assert_eq!(What::parse("*"), Some(What::Any));
        assert_eq!(What::parse("tier:commit"), Some(What::UpTo(Tier::Commit)));
        assert_eq!(What::parse("submit"), Some(What::Action(Action::Submit)));
        assert_eq!(
            What::parse("useConnector:github"),
            Some(What::Action(Action::UseConnector("github".into())))
        );
        assert_eq!(What::parse("tier:nope"), None);
        assert_eq!(What::parse("nope"), None);
    }

    #[test]
    fn tier_patterns_cover_lower_site_tiers_only() {
        let act = What::UpTo(Tier::Act);
        assert!(act.covers_action(&Action::ReadPage));
        assert!(act.covers_action(&Action::Interact));
        assert!(!act.covers_action(&Action::Submit));
        assert!(!act.covers_action(&Action::UseConnector("x".into())));
        assert!(!act.covers_action(&Action::ReadCredentials));

        let purchase = What::UpTo(Tier::Purchase);
        assert!(purchase.covers_action(&Action::Purchase));
        assert!(!purchase.covers_action(&Action::UseConnector("x".into())));
        assert!(!purchase.covers_action(&Action::ReadCredentials));

        let connector = What::UpTo(Tier::Connector);
        assert!(connector.covers_action(&Action::UseConnector("x".into())));
        assert!(!connector.covers_action(&Action::ReadPage));
    }

    #[test]
    fn agent_rules_match_agent_action_and_site() {
        let rule = agent_rule("tier:act", "github.com", Effect::Allow);
        let gh = origin("https://github.com");
        assert!(rule.covers_agent("ion", &Action::Interact, Some(&gh), None));
        assert!(!rule.covers_agent("codex", &Action::Interact, Some(&gh), None));
        assert!(!rule.covers_agent("ion", &Action::Submit, Some(&gh), None));
        assert!(!rule.covers_agent(
            "ion",
            &Action::Interact,
            Some(&origin("https://gitlab.com")),
            None
        ));
        assert!(!rule.covers_site(&gh, SiteCapability::Camera, None));
    }

    #[test]
    fn tab_rules_only_cover_their_tab() {
        let mut rule = agent_rule("*", "*", Effect::Allow);
        rule.lifetime = Lifetime::Tab(4);
        let site = origin("https://a.example");
        assert!(rule.covers_agent("ion", &Action::ReadPage, Some(&site), Some(4)));
        assert!(!rule.covers_agent("ion", &Action::ReadPage, Some(&site), Some(5)));
        assert!(!rule.covers_agent("ion", &Action::ReadPage, Some(&site), None));
    }

    #[test]
    fn site_rules_match_capability_and_origin() {
        let rule = Rule {
            who: Who::Site,
            what: What::Site(SiteCapability::Camera),
            site: SitePattern::Origin(origin("https://meet.example")),
            effect: Effect::Allow,
            lifetime: Lifetime::Forever,
            source: Source::User,
            created: 0,
        };
        assert!(rule.covers_site(
            &origin("https://meet.example/room"),
            SiteCapability::Camera,
            None
        ));
        assert!(!rule.covers_site(
            &origin("https://meet.example"),
            SiteCapability::Microphone,
            None
        ));
        assert!(!rule.covers_site(&origin("http://meet.example"), SiteCapability::Camera, None));
        assert!(!rule.covers_agent(
            "ion",
            &Action::ReadPage,
            Some(&origin("https://meet.example")),
            None
        ));
    }

    #[test]
    fn the_most_specific_rule_decides() {
        let rules = [
            agent_rule("*", "*", Effect::Deny),
            agent_rule("tier:act", "github.com", Effect::Allow),
            agent_rule("interact", "github.com", Effect::Ask),
            agent_rule("interact", "gist.github.com", Effect::Allow),
        ];
        let pick = |action: Action, site: &str| {
            let site = origin(site);
            deciding(
                rules
                    .iter()
                    .filter(|r| r.covers_agent("ion", &action, Some(&site), None)),
            )
            .map(|r| r.effect)
        };
        assert_eq!(
            pick(Action::ReadPage, "https://github.com"),
            Some(Effect::Allow)
        );
        assert_eq!(
            pick(Action::Interact, "https://github.com"),
            Some(Effect::Ask)
        );
        assert_eq!(
            pick(Action::Interact, "https://gist.github.com"),
            Some(Effect::Allow)
        );
        assert_eq!(
            pick(Action::ReadPage, "https://example.com"),
            Some(Effect::Deny)
        );
    }

    #[test]
    fn deny_wins_a_tie() {
        let rules = [
            agent_rule("submit", "github.com", Effect::Allow),
            agent_rule("submit", "github.com", Effect::Deny),
            agent_rule("submit", "github.com", Effect::Ask),
        ];
        assert_eq!(deciding(rules.iter()).map(|r| r.effect), Some(Effect::Deny));
        assert_eq!(
            deciding(rules[..1].iter()).map(|r| r.effect),
            Some(Effect::Allow)
        );
        assert_eq!(
            deciding(rules[2..].iter()).map(|r| r.effect),
            Some(Effect::Ask)
        );
    }

    #[test]
    fn action_specificity_beats_site_specificity() {
        // An exact action on any site outranks a tier on an exact origin.
        let rules = [
            agent_rule("tier:commit", "https://github.com", Effect::Allow),
            agent_rule("submit", "*", Effect::Deny),
        ];
        assert_eq!(deciding(rules.iter()).map(|r| r.effect), Some(Effect::Deny));
    }

    #[test]
    fn rules_serialize() {
        let rule = agent_rule("tier:act", "github.com", Effect::Allow);
        let json = serde_json::to_string(&rule).unwrap();
        assert_eq!(serde_json::from_str::<Rule>(&json).unwrap(), rule);
    }
}
