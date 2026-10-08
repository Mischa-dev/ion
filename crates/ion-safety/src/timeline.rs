//! The activity log and remembered agent rules in words, for the agent
//! activity view: "Ion Agent typed “Mischa” into Name on lumen.example",
//! "You let Ion Agent read and work on github.com".
//!
//! Only what people want to look back on: what agents did, what the person
//! answered, blocks, and stops. Routine "allowed" decisions are left out,
//! since the action that follows says the same thing.

use serde::Serialize;

use crate::audit::{Entry, Kind};
use crate::capability::{Action, Tier};
use crate::policy::Verdict;
use crate::prompt::{agent_phrase, site_name};
use crate::rule::{Effect, Lifetime, Rule, What, Who};
use crate::site::{Origin, SitePattern};

/// How a line reads at a glance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Mark {
    /// An agent did something.
    Done,
    /// An agent tried and it didn't work.
    Failed,
    /// The person allowed something.
    Allowed,
    /// The person or Ion refused something.
    Refused,
    /// Stops, take-overs and other notes.
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    /// Unix seconds.
    pub time: u64,
    pub text: String,
    pub mark: Mark,
    /// The agent's task, to group a task's lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
}

/// How a logged target (an origin, or a connector name) is named.
fn target_name(target: Option<&str>) -> String {
    site_name(target.and_then(Origin::parse).as_ref())
}

/// `entry` as a line, or `None` for entries the view leaves out. `name`
/// gives an agent's display name from its id.
pub fn describe(entry: &Entry, name: &dyn Fn(&str) -> String) -> Option<Line> {
    let agent = entry.agent.as_deref().map(name);
    let agent = agent.as_deref().unwrap_or("An agent");
    let site = target_name(entry.target.as_deref());
    let phrase = entry.action.as_ref().map(|a| agent_phrase(a, &site));
    let detail = entry.detail.as_deref().unwrap_or("").trim();
    let (text, mark) = match entry.kind {
        Kind::Action => {
            let ok = entry.ok.unwrap_or(true);
            let what = if detail.is_empty() {
                phrase.unwrap_or_default()
            } else {
                lowercase_first(detail)
            };
            let text = if ok {
                format!("{agent}: {what}")
            } else {
                format!("{agent} couldn't {what}")
            };
            (text, if ok { Mark::Done } else { Mark::Failed })
        }
        Kind::Answer => {
            // Site permission answers belong to the site's own panel.
            let phrase = phrase?;
            match (entry.verdict, detail) {
                (Some(Verdict::Deny), _) => {
                    (format!("You didn't let {agent} {phrase}"), Mark::Refused)
                }
                (_, "allowOnce") => (format!("You let {agent} {phrase}, once"), Mark::Allowed),
                // Allowing a read on a site covers working there too.
                _ if entry.action == Some(Action::ReadPage) => (
                    format!("You let {agent} read and work on {site}"),
                    Mark::Allowed,
                ),
                _ => (format!("You let {agent} {phrase}"), Mark::Allowed),
            }
        }
        Kind::Decision => {
            if entry.verdict != Some(Verdict::Deny) {
                return None;
            }
            let phrase = phrase?;
            (
                format!("Ion stopped {agent} from trying to {phrase}"),
                Mark::Refused,
            )
        }
        Kind::Forget => (
            format!("You took back a permission for {agent}"),
            Mark::Note,
        ),
        Kind::Stop => ("You stopped all agents".to_owned(), Mark::Note),
        Kind::Resume => ("You let agents run again".to_owned(), Mark::Note),
        Kind::TakeOver => ("You took over a tab".to_owned(), Mark::Note),
        Kind::HandBack => ("You handed a tab back".to_owned(), Mark::Note),
    };
    Some(Line {
        time: entry.time,
        text,
        mark,
        task: entry.task.clone(),
    })
}

/// The log as lines, newest first.
pub fn lines(entries: &[Entry], name: &dyn Fn(&str) -> String) -> Vec<Line> {
    entries
        .iter()
        .rev()
        .filter_map(|e| describe(e, name))
        .collect()
}

/// A remembered agent rule in words: "Can read and work on github.com",
/// "Can't send or submit something on any site, until Ion quits".
pub fn rule_text(rule: &Rule) -> Option<String> {
    if !matches!(rule.who, Who::Agent(_)) {
        return None;
    }
    let site = match &rule.site {
        SitePattern::Any => "any site".to_owned(),
        SitePattern::Domain(domain) => domain.clone(),
        SitePattern::Origin(origin) => site_name(Some(origin)),
    };
    let what = match &rule.what {
        What::Action(action) => agent_phrase(action, &site),
        What::UpTo(Tier::Read) => format!("read {site}"),
        What::UpTo(Tier::Act) => format!("read and work on {site}"),
        What::UpTo(tier) => format!("do {} things on {site}", tier_name(*tier)),
        What::Any => format!("do anything on {site}"),
        What::Site(_) => return None,
    };
    let can = match rule.effect {
        Effect::Allow => "Can",
        Effect::Deny => "Can't",
        Effect::Ask => "Asks before it can",
    };
    let until = match rule.lifetime {
        Lifetime::Forever => "",
        Lifetime::Session => ", until Ion quits",
        Lifetime::Tab(_) => ", in one tab",
        Lifetime::Once => ", once",
    };
    Some(format!("{can} {what}{until}"))
}

fn tier_name(tier: Tier) -> &'static str {
    match tier {
        Tier::Read => "read-only",
        Tier::Act => "everyday",
        Tier::Commit => "sending",
        Tier::Personal => "personal",
        Tier::Purchase => "purchase",
        Tier::Connector => "connector",
        Tier::Forbidden => "forbidden",
    }
}

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if !text.starts_with("I ") => first.to_lowercase().chain(chars).collect(),
        _ => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::Source;

    fn name(id: &str) -> String {
        if id == "ion" {
            "Ion Agent".into()
        } else {
            id.into()
        }
    }

    fn agent_entry(kind: Kind, action: Action) -> Entry {
        let mut e = Entry::new(100, kind);
        e.agent = Some("ion".into());
        e.action = Some(action);
        e.target = Some("https://www.github.com".into());
        e
    }

    #[test]
    fn actions_use_their_detail() {
        let mut e = agent_entry(Kind::Action, Action::Interact);
        e.ok = Some(true);
        e.detail = Some("Type “Mischa” into Name".into());
        let line = describe(&e, &name).unwrap();
        assert_eq!(line.text, "Ion Agent: type “Mischa” into Name");
        assert_eq!(line.mark, Mark::Done);

        e.ok = Some(false);
        e.detail = None;
        let line = describe(&e, &name).unwrap();
        assert_eq!(line.text, "Ion Agent couldn't work on github.com");
        assert_eq!(line.mark, Mark::Failed);
    }

    #[test]
    fn answers_say_what_the_person_chose() {
        let mut e = agent_entry(Kind::Answer, Action::ReadPage);
        e.verdict = Some(Verdict::Allow);
        e.detail = Some("allow".into());
        assert_eq!(
            describe(&e, &name).unwrap().text,
            "You let Ion Agent read and work on github.com"
        );
        e.detail = Some("allowOnce".into());
        assert_eq!(
            describe(&e, &name).unwrap().text,
            "You let Ion Agent read github.com, once"
        );
        e.verdict = Some(Verdict::Deny);
        e.detail = Some("deny".into());
        let line = describe(&e, &name).unwrap();
        assert_eq!(line.text, "You didn't let Ion Agent read github.com");
        assert_eq!(line.mark, Mark::Refused);
    }

    #[test]
    fn routine_decisions_and_site_answers_are_left_out() {
        let mut e = agent_entry(Kind::Decision, Action::ReadPage);
        e.verdict = Some(Verdict::Allow);
        assert_eq!(describe(&e, &name), None);
        e.verdict = Some(Verdict::Ask);
        assert_eq!(describe(&e, &name), None);
        e.verdict = Some(Verdict::Deny);
        assert_eq!(
            describe(&e, &name).unwrap().text,
            "Ion stopped Ion Agent from trying to read github.com"
        );

        let mut site = Entry::new(100, Kind::Answer);
        site.target = Some("https://github.com".into());
        site.verdict = Some(Verdict::Allow);
        assert_eq!(describe(&site, &name), None);
    }

    #[test]
    fn newest_first() {
        let mut a = Entry::new(1, Kind::Stop);
        a.time = 1;
        let mut b = Entry::new(2, Kind::Resume);
        b.time = 2;
        let texts: Vec<String> = lines(&[a, b], &name).into_iter().map(|l| l.text).collect();
        assert_eq!(
            texts,
            ["You let agents run again", "You stopped all agents"]
        );
    }

    #[test]
    fn rules_read_as_sentences() {
        let rule = Rule {
            who: Who::Agent("ion".into()),
            what: What::UpTo(Tier::Act),
            site: SitePattern::Domain("github.com".into()),
            effect: Effect::Allow,
            lifetime: Lifetime::Forever,
            source: Source::User,
            created: 0,
        };
        assert_eq!(rule_text(&rule).unwrap(), "Can read and work on github.com");
        let denied = Rule {
            what: What::Action(Action::Submit),
            site: SitePattern::Any,
            effect: Effect::Deny,
            lifetime: Lifetime::Session,
            ..rule.clone()
        };
        assert_eq!(
            rule_text(&denied).unwrap(),
            "Can't send or submit something on any site, until Ion quits"
        );
        let site_rule = Rule {
            who: Who::Site,
            ..rule
        };
        assert_eq!(rule_text(&site_rule), None);
    }
}
