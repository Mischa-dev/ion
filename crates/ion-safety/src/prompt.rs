//! Prompts: what Ion asks when a decision is "ask", and what an answer means.
//!
//! Sentences are built from Ion's own words, the agent's configured name and
//! the site's host only, never from page titles or text an agent supplies, so
//! neither a page nor an agent can write the prompt.

use serde::{Deserialize, Serialize};

use crate::capability::{Action, Tier};
use crate::policy::{AgentRequest, SiteRequest, Verdict};
use crate::rule::{Effect, Lifetime, Rule, Source, What, Who};
use crate::site::{Origin, SitePattern};

/// What a prompt is about.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Subject {
    Site(SiteRequest),
    Agent(AgentRequest),
}

/// An answer to a prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Choice {
    /// Allow and remember.
    Allow,
    /// Allow this one request.
    AllowOnce,
    /// Refuse (and remember: forever for sites, this session for agents).
    Deny,
}

impl Choice {
    pub fn id(self) -> &'static str {
        match self {
            Choice::Allow => "allow",
            Choice::AllowOnce => "allowOnce",
            Choice::Deny => "deny",
        }
    }

    pub fn from_id(id: &str) -> Option<Choice> {
        [Choice::Allow, Choice::AllowOnce, Choice::Deny]
            .into_iter()
            .find(|c| c.id() == id)
    }
}

/// A question for the person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub subject: Subject,
    /// "Ion Agent wants to send or submit something on github.com".
    pub text: String,
    /// The answers offered, with their button labels, in display order; the
    /// last is the primary button.
    pub choices: Vec<(Choice, String)>,
}

/// How a page is named in prompts.
fn site_name(site: Option<&Origin>) -> String {
    match site {
        Some(o) => match o.display_host() {
            Some(host) => host.to_owned(),
            None if o.scheme() == "file" => "a local file".to_owned(),
            None => "this page".to_owned(),
        },
        None => "this page".to_owned(),
    }
}

fn agent_phrase(action: &Action, site: &str) -> String {
    match action {
        Action::ReadPage => format!("read {site}"),
        Action::ListTabs => "see your open tabs".to_owned(),
        Action::Navigate => format!("open {site}"),
        Action::OpenTab => format!("open {site} in a new tab"),
        Action::CloseTab => format!("close a tab on {site}"),
        Action::Interact => format!("work on {site}"),
        Action::DevTools => format!("use developer tools on {site}"),
        Action::Download => format!("download a file from {site}"),
        Action::Submit => format!("send or submit something on {site}"),
        Action::UploadFile => format!("upload one of your files to {site}"),
        Action::EnterPersonalData => format!("enter your personal details on {site}"),
        Action::Purchase => format!("make a purchase on {site}"),
        Action::UseConnector(name) => format!("use {name}"),
        Action::ReadCredentials => "see your saved passwords".to_owned(),
    }
}

impl Prompt {
    pub fn for_site(request: SiteRequest) -> Prompt {
        let who = match request.origin.display_host() {
            Some(host) => host.to_owned(),
            None => "This page".to_owned(),
        };
        Prompt {
            text: format!("{who} wants to {}", request.capability.request_phrase()),
            choices: vec![
                (Choice::Deny, "Block".to_owned()),
                (Choice::AllowOnce, "Allow this time".to_owned()),
                (Choice::Allow, "Allow".to_owned()),
            ],
            subject: Subject::Site(request),
        }
    }

    /// `agent_name` is the display name from the agent's profile.
    pub fn for_agent(request: AgentRequest, agent_name: &str) -> Prompt {
        let site = site_name(request.site.as_ref());
        let text = format!(
            "{agent_name} wants to {}",
            agent_phrase(&request.action, &site)
        );
        let deny = (Choice::Deny, "Don't allow".to_owned());
        let once = (Choice::AllowOnce, "Allow once".to_owned());
        let tier = request.action.tier();
        let choices = match tier {
            Tier::Read | Tier::Act if request.site.is_some() => {
                vec![deny, once, (Choice::Allow, format!("Allow on {site}"))]
            }
            Tier::Commit => vec![
                deny,
                (Choice::Allow, format!("Always allow on {site}")),
                once,
            ],
            // Personal data and purchases are never remembered below full trust.
            Tier::Personal | Tier::Purchase | Tier::Forbidden => vec![deny, once],
            _ => vec![deny, once, (Choice::Allow, "Allow".to_owned())],
        };
        Prompt {
            text,
            choices,
            subject: Subject::Agent(request),
        }
    }

    pub fn offers(&self, choice: Choice) -> bool {
        self.choices.iter().any(|(c, _)| *c == choice)
    }

    /// What `choice` means: the verdict for the waiting request and the rule
    /// to remember, if any. `None` if the prompt does not offer `choice`.
    pub fn resolve(&self, choice: Choice, now: u64) -> Option<(Verdict, Option<Rule>)> {
        if !self.offers(choice) {
            return None;
        }
        let verdict = match choice {
            Choice::Allow | Choice::AllowOnce => Verdict::Allow,
            Choice::Deny => Verdict::Deny,
        };
        if choice == Choice::AllowOnce {
            return Some((verdict, None));
        }
        let effect = if choice == Choice::Deny {
            Effect::Deny
        } else {
            Effect::Allow
        };
        let rule = match &self.subject {
            Subject::Site(request) => Rule {
                who: Who::Site,
                what: What::Site(request.capability),
                site: SitePattern::Origin(request.origin.clone()),
                effect,
                lifetime: Lifetime::Forever,
                source: Source::User,
                created: now,
            },
            Subject::Agent(request) => {
                let tier = request.action.tier();
                // Approving a site covers reading and acting there. Anything
                // not about a site (listing tabs, a connector) covers only
                // that action.
                let what = match tier {
                    Tier::Read | Tier::Act if request.site.is_some() => What::UpTo(Tier::Act),
                    _ => What::Action(request.action.clone()),
                };
                let site = match (&request.site, tier.is_site_tier()) {
                    (Some(origin), true) => SitePattern::for_origin_host(origin),
                    _ => SitePattern::Any,
                };
                let local = request.site.as_ref().is_some_and(|o| o.host().is_none());
                let lifetime = if effect == Effect::Deny || local {
                    Lifetime::Session
                } else {
                    Lifetime::Forever
                };
                Rule {
                    who: Who::Agent(request.agent.clone()),
                    what,
                    site,
                    effect,
                    lifetime,
                    source: Source::User,
                    created: now,
                }
            }
        };
        Some((verdict, Some(rule)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::SiteCapability;

    fn agent(action: Action, site: Option<&str>) -> Prompt {
        Prompt::for_agent(
            AgentRequest {
                agent: "ion".into(),
                action,
                site: site.map(|s| Origin::parse(s).unwrap()),
                tab: Some(1),
            },
            "Ion Agent",
        )
    }

    fn choices(p: &Prompt) -> Vec<Choice> {
        p.choices.iter().map(|(c, _)| *c).collect()
    }

    #[test]
    fn site_prompts_name_the_host() {
        let p = Prompt::for_site(SiteRequest {
            origin: Origin::parse("https://www.maps.example/x").unwrap(),
            capability: SiteCapability::Location,
            tab: None,
        });
        assert_eq!(p.text, "maps.example wants to know your location");
        assert_eq!(
            choices(&p),
            [Choice::Deny, Choice::AllowOnce, Choice::Allow]
        );

        let local = Prompt::for_site(SiteRequest {
            origin: Origin::parse("file:///home/me/a.html").unwrap(),
            capability: SiteCapability::Notifications,
            tab: None,
        });
        assert_eq!(local.text, "This page wants to show notifications");
    }

    #[test]
    fn agent_prompts_read_naturally() {
        assert_eq!(
            agent(Action::Submit, Some("https://www.github.com")).text,
            "Ion Agent wants to send or submit something on github.com"
        );
        assert_eq!(
            agent(Action::ReadPage, Some("file:///notes.html")).text,
            "Ion Agent wants to read a local file"
        );
        assert_eq!(
            agent(Action::UseConnector("GitHub".into()), None).text,
            "Ion Agent wants to use GitHub"
        );
        assert_eq!(
            agent(Action::ListTabs, None).text,
            "Ion Agent wants to see your open tabs"
        );
    }

    #[test]
    fn purchases_cannot_be_remembered() {
        let p = agent(Action::Purchase, Some("https://shop.example"));
        assert_eq!(choices(&p), [Choice::Deny, Choice::AllowOnce]);
        assert_eq!(p.resolve(Choice::Allow, 0), None);
        assert_eq!(
            p.resolve(Choice::AllowOnce, 0),
            Some((Verdict::Allow, None))
        );
    }

    #[test]
    fn approving_a_site_covers_read_and_act_on_its_host() {
        let p = agent(Action::ReadPage, Some("https://github.com/x"));
        let (verdict, rule) = p.resolve(Choice::Allow, 42).unwrap();
        let rule = rule.unwrap();
        assert_eq!(verdict, Verdict::Allow);
        assert_eq!(rule.what, What::UpTo(Tier::Act));
        assert_eq!(rule.site, SitePattern::Domain("github.com".into()));
        assert_eq!(rule.lifetime, Lifetime::Forever);
        assert_eq!(rule.created, 42);
        assert_eq!(rule.who, Who::Agent("ion".into()));
    }

    #[test]
    fn commit_answers_cover_that_action_only() {
        let p = agent(Action::Submit, Some("https://github.com"));
        let (_, rule) = p.resolve(Choice::Allow, 0).unwrap();
        assert_eq!(rule.unwrap().what, What::Action(Action::Submit));
    }

    #[test]
    fn agent_denials_last_the_session() {
        let p = agent(Action::Interact, Some("https://github.com"));
        let (verdict, rule) = p.resolve(Choice::Deny, 0).unwrap();
        assert_eq!(verdict, Verdict::Deny);
        let rule = rule.unwrap();
        assert_eq!(rule.lifetime, Lifetime::Session);
        assert_eq!(rule.effect, Effect::Deny);
    }

    #[test]
    fn local_page_approvals_last_the_session() {
        let p = agent(Action::ReadPage, Some("file:///notes.html"));
        let (_, rule) = p.resolve(Choice::Allow, 0).unwrap();
        let rule = rule.unwrap();
        assert_eq!(rule.lifetime, Lifetime::Session);
        assert_eq!(
            rule.site,
            SitePattern::Origin(Origin::parse("file:///").unwrap())
        );
    }

    #[test]
    fn connector_answers_apply_everywhere() {
        let p = agent(Action::UseConnector("github".into()), None);
        let (_, rule) = p.resolve(Choice::Allow, 0).unwrap();
        let rule = rule.unwrap();
        assert_eq!(rule.site, SitePattern::Any);
        assert_eq!(
            rule.what,
            What::Action(Action::UseConnector("github".into()))
        );
    }

    #[test]
    fn site_answers_are_per_origin_and_forever() {
        let p = Prompt::for_site(SiteRequest {
            origin: Origin::parse("https://meet.example").unwrap(),
            capability: SiteCapability::Camera,
            tab: Some(2),
        });
        let (verdict, rule) = p.resolve(Choice::Deny, 0).unwrap();
        let rule = rule.unwrap();
        assert_eq!(verdict, Verdict::Deny);
        assert_eq!(rule.who, Who::Site);
        assert_eq!(rule.lifetime, Lifetime::Forever);
        assert_eq!(
            rule.site,
            SitePattern::Origin(Origin::parse("https://meet.example").unwrap())
        );
    }

    #[test]
    fn listing_tabs_approves_only_listing_tabs() {
        let p = agent(Action::ListTabs, None);
        let (_, rule) = p.resolve(Choice::Allow, 0).unwrap();
        let rule = rule.unwrap();
        assert_eq!(rule.what, What::Action(Action::ListTabs));
        assert_eq!(rule.site, SitePattern::Any);
    }

    #[test]
    fn choice_ids_round_trip() {
        for c in [Choice::Allow, Choice::AllowOnce, Choice::Deny] {
            assert_eq!(Choice::from_id(c.id()), Some(c));
        }
        assert_eq!(Choice::from_id("maybe"), None);
    }
}
