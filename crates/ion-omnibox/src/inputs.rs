//! Input history: which page people chose after typing what.
//!
//! Each time a suggestion is picked, the typed text and the chosen URL are
//! remembered. Later, typing the start of that text ranks the page higher
//! (and lets the address bar fill it in), even when the text doesn't appear
//! in the page's address or title: `gm` can mean `mail.google.com`. Like
//! Firefox's adaptive history, this is how the address bar learns.

use serde::{Deserialize, Serialize};

use crate::suggest::Page;

/// Choices kept before the least used are dropped.
pub const MAX_CHOICES: usize = 1000;
/// Typed text longer than this is a search, not a shortcut worth learning.
const MAX_INPUT: usize = 64;
const DAY_SECS: u64 = 24 * 60 * 60;

/// One remembered choice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    /// What was typed, lowercase and trimmed.
    pub input: String,
    pub url: String,
    /// The page's title when it was chosen.
    #[serde(default)]
    pub title: String,
    pub uses: u32,
    /// Unix time (seconds) of the latest use.
    pub last_used: u64,
}

impl Choice {
    fn weight(&self, now: u64) -> f64 {
        let age_days = now.saturating_sub(self.last_used) / DAY_SECS;
        let recency = match age_days {
            0..=7 => 1.0,
            8..=30 => 0.7,
            31..=90 => 0.4,
            _ => 0.15,
        };
        f64::from(self.uses.min(10)) * recency
    }
}

/// The remembered choices.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InputHistory {
    #[serde(default)]
    choices: Vec<Choice>,
}

fn normalize(input: &str) -> Option<String> {
    let input = input.trim().to_lowercase();
    (!input.is_empty() && input.chars().count() <= MAX_INPUT).then_some(input)
}

impl InputHistory {
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_owned())
    }

    pub fn len(&self) -> usize {
        self.choices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.choices.is_empty()
    }

    /// Remember that `url` was chosen after typing `input`. False when
    /// there was nothing worth remembering.
    pub fn record(&mut self, input: &str, url: &str, title: &str, now: u64) -> bool {
        let Some(input) = normalize(input) else {
            return false;
        };
        if url.is_empty() {
            return false;
        }
        match self
            .choices
            .iter_mut()
            .find(|c| c.input == input && c.url == url)
        {
            Some(choice) => {
                choice.uses = choice.uses.saturating_add(1);
                choice.last_used = choice.last_used.max(now);
                if !title.is_empty() {
                    choice.title = title.to_owned();
                }
            }
            None => self.choices.push(Choice {
                input,
                url: url.to_owned(),
                title: title.to_owned(),
                uses: 1,
                last_used: now,
            }),
        }
        if self.choices.len() > MAX_CHOICES {
            self.choices
                .sort_by(|a, b| b.weight(now).total_cmp(&a.weight(now)));
            self.choices.truncate(MAX_CHOICES * 9 / 10);
        }
        true
    }

    /// Pages chosen before after typing text that starts with `query`,
    /// strongest first; each page's `frecency` is how strongly it's preferred.
    pub fn boosts(&self, query: &str, now: u64) -> Vec<Page> {
        let Some(query) = normalize(query) else {
            return Vec::new();
        };
        let mut boosts: Vec<Page> = Vec::new();
        for choice in self.choices.iter().filter(|c| c.input.starts_with(&query)) {
            // Typing all of "github" counts more than just its "g".
            let coverage = query.chars().count() as f64 / choice.input.chars().count() as f64;
            let weight = choice.weight(now) * (0.5 + coverage / 2.0);
            match boosts.iter_mut().find(|page| page.url == choice.url) {
                Some(page) => page.frecency += weight,
                None => boosts.push(Page {
                    title: choice.title.clone(),
                    url: choice.url.clone(),
                    frecency: weight,
                }),
            }
        }
        boosts.sort_by(|a, b| b.frecency.total_cmp(&a.frecency));
        boosts
    }

    /// Forget every choice of `url`, as when it's removed from history.
    pub fn forget_url(&mut self, url: &str) -> bool {
        let before = self.choices.len();
        self.choices.retain(|c| c.url != url);
        self.choices.len() != before
    }

    /// Forget choices of pages on `site` and its subdomains, as history's
    /// "forget this site" does. True if any went.
    pub fn forget_site(&mut self, site: &str) -> bool {
        let site = site.to_lowercase();
        let site = site.strip_prefix("www.").unwrap_or(&site).to_owned();
        if site.is_empty() {
            return false;
        }
        let before = self.choices.len();
        self.choices.retain(|c| {
            let host = url::Url::parse(&c.url)
                .ok()
                .and_then(|url| url.host_str().map(str::to_lowercase))
                .unwrap_or_default();
            let host = host.strip_prefix("www.").unwrap_or(&host);
            !(host == site || host.ends_with(&format!(".{site}")))
        });
        self.choices.len() != before
    }

    pub fn clear(&mut self) {
        self.choices.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_800_000_000;
    const MAIL: &str = "https://mail.google.com/";

    #[test]
    fn learns_from_choices() {
        let mut inputs = InputHistory::default();
        assert!(inputs.record("GMail ", MAIL, "Gmail", NOW));
        assert!(inputs.record("gmail", MAIL, "", NOW));
        assert!(inputs.record("gmail", "https://gmail.example/", "", NOW));
        assert!(!inputs.record("  ", MAIL, "", NOW));
        assert_eq!(inputs.len(), 2);

        let boosts = inputs.boosts("gm", NOW);
        assert_eq!(boosts[0].url, MAIL);
        assert_eq!(boosts[0].title, "Gmail");
        assert!(boosts[0].frecency > boosts[1].frecency);
        assert!(inputs.boosts("x", NOW).is_empty());
        // Typing more than was typed before isn't a match.
        assert!(inputs.boosts("gmailx", NOW).is_empty());
    }

    #[test]
    fn full_input_counts_more_than_a_prefix() {
        let mut inputs = InputHistory::default();
        inputs.record("github", "https://github.com/", "", NOW);
        let full = inputs.boosts("github", NOW)[0].frecency;
        let prefix = inputs.boosts("g", NOW)[0].frecency;
        assert!(full > prefix);
    }

    #[test]
    fn old_choices_fade() {
        let mut inputs = InputHistory::default();
        inputs.record("news", "https://old.example/", "", NOW - 200 * DAY_SECS);
        inputs.record("news", "https://new.example/", "", NOW);
        assert_eq!(inputs.boosts("news", NOW)[0].url, "https://new.example/");
    }

    #[test]
    fn forgets_and_round_trips() {
        let mut inputs = InputHistory::default();
        inputs.record("gm", MAIL, "", NOW);
        let copy = InputHistory::from_json(&inputs.to_json()).unwrap();
        assert_eq!(copy, inputs);
        assert!(inputs.forget_url(MAIL));
        assert!(inputs.is_empty());
        assert_eq!(InputHistory::from_json("{}").unwrap().len(), 0);
    }

    #[test]
    fn forgetting_a_site_takes_its_subdomains() {
        let mut inputs = InputHistory::default();
        inputs.record("docs", "https://docs.example.com/a", "Docs", 1);
        inputs.record("ex", "https://www.example.com/", "Example", 1);
        inputs.record("notex", "https://notexample.com/", "Other", 1);
        assert!(inputs.forget_site("example.com"));
        assert!(inputs.boosts("docs", 2).is_empty());
        assert!(inputs.boosts("ex", 2).is_empty());
        assert_eq!(inputs.boosts("notex", 2).len(), 1);
        assert!(!inputs.forget_site("example.com"));
    }
}
