//! `!name query` shortcuts typed in the URL bar or the palette.
//!
//! A bang maps a short trigger to a URL template containing `{}`, for example
//! `gh` → `https://github.com/search?q={}`. The bang can come first
//! (`!gh cxx-qt`) or last (`cxx-qt !gh`); a bang with no query opens the
//! site's front page. Unknown bangs are left alone, so they fall through to
//! the search engine (DuckDuckGo handles its own bangs).

use std::collections::BTreeMap;
use std::fmt;

use ion_core::navigation::{InputStep, Resolved};
use url::Url;

/// Built-in bangs: (trigger, name, template).
const DEFAULTS: &[(&str, &str, &str)] = &[
    ("a", "Amazon", "https://www.amazon.com/s?k={}"),
    (
        "aw",
        "ArchWiki",
        "https://wiki.archlinux.org/index.php?search={}",
    ),
    ("crates", "crates.io", "https://crates.io/search?q={}"),
    ("ddg", "DuckDuckGo", "https://duckduckgo.com/?q={}"),
    ("g", "Google", "https://www.google.com/search?q={}"),
    ("gh", "GitHub", "https://github.com/search?q={}"),
    (
        "hm",
        "Home Manager options",
        "https://home-manager-options.extranix.com/?query={}",
    ),
    ("hn", "Hacker News", "https://hn.algolia.com/?q={}"),
    (
        "i",
        "Image search",
        "https://duckduckgo.com/?ia=images&iax=images&q={}",
    ),
    (
        "maps",
        "OpenStreetMap",
        "https://www.openstreetmap.org/search?query={}",
    ),
    (
        "mdn",
        "MDN Web Docs",
        "https://developer.mozilla.org/en-US/search?q={}",
    ),
    (
        "nix",
        "Nix packages",
        "https://search.nixos.org/packages?query={}",
    ),
    (
        "nixopt",
        "NixOS options",
        "https://search.nixos.org/options?query={}",
    ),
    ("npm", "npm", "https://www.npmjs.com/search?q={}"),
    ("r", "Reddit", "https://www.reddit.com/search/?q={}"),
    ("rs", "docs.rs", "https://docs.rs/releases/search?query={}"),
    (
        "rust",
        "Rust std docs",
        "https://doc.rust-lang.org/std/?search={}",
    ),
    (
        "so",
        "Stack Overflow",
        "https://stackoverflow.com/search?q={}",
    ),
    (
        "w",
        "Wikipedia",
        "https://en.wikipedia.org/w/index.php?search={}",
    ),
    (
        "wa",
        "Wolfram|Alpha",
        "https://www.wolframalpha.com/input?i={}",
    ),
    (
        "yt",
        "YouTube",
        "https://www.youtube.com/results?search_query={}",
    ),
];

/// One `!trigger` and where it leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bang {
    /// What follows the `!`, lowercase.
    pub trigger: String,
    /// Human-facing name shown in the palette.
    pub name: String,
    /// URL with `{}` where the encoded query goes.
    pub template: String,
}

impl Bang {
    /// URL for `query`; the site's front page when the query is empty.
    pub fn url_for(&self, query: &str) -> String {
        let query = query.trim();
        if query.is_empty() {
            return self.home();
        }
        let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
        self.template.replace("{}", &encoded)
    }

    /// The origin of the template, e.g. `https://github.com/`.
    pub fn home(&self) -> String {
        Url::parse(&self.template.replace("{}", ""))
            .map(|url| format!("{}/", url.origin().ascii_serialization()))
            .unwrap_or_else(|_| self.template.replace("{}", ""))
    }
}

/// Why a bang from config was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BangError {
    /// Empty, or contains whitespace or `!`.
    InvalidTrigger(String),
    /// Not an http(s) URL.
    InvalidTemplate { trigger: String, template: String },
}

impl fmt::Display for BangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BangError::InvalidTrigger(t) => write!(f, "invalid bang trigger {t:?}"),
            BangError::InvalidTemplate { trigger, template } => {
                write!(f, "bang !{trigger}: {template:?} is not an http(s) URL")
            }
        }
    }
}

impl std::error::Error for BangError {}

/// The set of known bangs, keyed by trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BangTable {
    bangs: BTreeMap<String, Bang>,
}

impl Default for BangTable {
    /// The built-in set.
    fn default() -> Self {
        let bangs = DEFAULTS
            .iter()
            .map(|&(trigger, name, template)| {
                let bang = Bang {
                    trigger: trigger.to_owned(),
                    name: name.to_owned(),
                    template: template.to_owned(),
                };
                (bang.trigger.clone(), bang)
            })
            .collect();
        Self { bangs }
    }
}

impl BangTable {
    /// A table with no bangs at all.
    pub fn empty() -> Self {
        Self {
            bangs: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.bangs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bangs.is_empty()
    }

    /// Look up a bang by trigger, ignoring case.
    pub fn get(&self, trigger: &str) -> Option<&Bang> {
        self.bangs.get(&trigger.to_lowercase())
    }

    /// All bangs, sorted by trigger.
    pub fn iter(&self) -> impl Iterator<Item = &Bang> {
        self.bangs.values()
    }

    /// Add or replace a bang. The display name is taken from the URL's host.
    pub fn insert(&mut self, trigger: &str, template: &str) -> Result<(), BangError> {
        let trigger = trigger.trim().trim_start_matches('!').to_lowercase();
        if trigger.is_empty() || trigger.contains(|c: char| c.is_whitespace() || c == '!') {
            return Err(BangError::InvalidTrigger(trigger));
        }
        let invalid = || BangError::InvalidTemplate {
            trigger: trigger.clone(),
            template: template.to_owned(),
        };
        let parsed = Url::parse(&template.replace("{}", "x")).map_err(|_| invalid())?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(invalid());
        }
        let host = parsed.host_str().ok_or_else(invalid)?;
        let name = host.strip_prefix("www.").unwrap_or(host).to_owned();
        self.bangs.insert(
            trigger.clone(),
            Bang {
                trigger,
                name,
                template: template.to_owned(),
            },
        );
        Ok(())
    }

    /// Remove a bang; returns whether it existed.
    pub fn remove(&mut self, trigger: &str) -> bool {
        self.bangs.remove(&trigger.to_lowercase()).is_some()
    }

    /// Apply a user table on top of this one, as written in config: each entry
    /// adds or replaces a bang, and an empty template removes it. Invalid
    /// entries are skipped and returned so the caller can report them.
    pub fn apply<'a>(
        &mut self,
        entries: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Vec<BangError> {
        let mut errors = Vec::new();
        for (trigger, template) in entries {
            if template.trim().is_empty() {
                self.remove(trigger.trim().trim_start_matches('!'));
            } else if let Err(err) = self.insert(trigger, template.trim()) {
                errors.push(err);
            }
        }
        errors
    }

    /// Find a known bang in `input`, first or last word. Returns the bang and
    /// the rest of the input as the query.
    pub fn parse<'a>(&self, input: &'a str) -> Option<(&Bang, &'a str)> {
        let input = input.trim();
        if let Some(rest) = input.strip_prefix('!') {
            let (trigger, query) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
            if let Some(bang) = self.get(trigger) {
                return Some((bang, query.trim()));
            }
        }
        let (query, last) = input.rsplit_once(char::is_whitespace)?;
        let bang = self.get(last.strip_prefix('!')?)?;
        Some((bang, query.trim()))
    }

    /// The URL `input` expands to, if it contains a known bang.
    pub fn expand(&self, input: &str) -> Option<String> {
        self.parse(input).map(|(bang, query)| bang.url_for(query))
    }
}

impl InputStep for BangTable {
    fn resolve(&self, input: &str) -> Option<Resolved> {
        self.expand(input).map(Resolved::Expanded)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ion_core::navigation::Omnibox;

    use super::*;

    fn expand(input: &str) -> Option<String> {
        BangTable::default().expand(input)
    }

    #[test]
    fn leading_bang_expands() {
        assert_eq!(
            expand("!gh cxx-qt"),
            Some("https://github.com/search?q=cxx-qt".into())
        );
        assert_eq!(
            expand("  !nix   ripgrep all "),
            Some("https://search.nixos.org/packages?query=ripgrep+all".into())
        );
    }

    #[test]
    fn trailing_bang_expands() {
        assert_eq!(
            expand("rust lifetimes !so"),
            Some("https://stackoverflow.com/search?q=rust+lifetimes".into())
        );
    }

    #[test]
    fn triggers_ignore_case() {
        assert_eq!(expand("!GH ion"), expand("!gh ion"));
    }

    #[test]
    fn bang_without_query_opens_front_page() {
        assert_eq!(expand("!gh"), Some("https://github.com/".into()));
        assert_eq!(expand("!yt"), Some("https://www.youtube.com/".into()));
    }

    #[test]
    fn query_is_encoded() {
        assert_eq!(
            expand("!ddg a&b=c #d"),
            Some("https://duckduckgo.com/?q=a%26b%3Dc+%23d".into())
        );
    }

    #[test]
    fn unknown_or_missing_bangs_are_ignored() {
        assert_eq!(expand("!nope thing"), None);
        assert_eq!(expand("!"), None);
        assert_eq!(expand("hello world"), None);
        assert_eq!(expand("hello!gh"), None);
        assert_eq!(expand("gh"), None);
    }

    #[test]
    fn config_entries_add_replace_and_remove() {
        let mut table = BangTable::default();
        let errors = table.apply([
            ("ion", "https://github.com/Mischa-dev/ion/issues?q={}"),
            ("!gh", "https://github.com/search?type=code&q={}"),
            ("yt", ""),
            ("bad name", "https://example.com/{}"),
            ("ftp", "ftp://example.com/{}"),
            ("nohost", "not a url {}"),
        ]);
        assert_eq!(errors.len(), 3);
        assert_eq!(table.get("ion").unwrap().name, "github.com");
        assert_eq!(
            table.expand("!gh x"),
            Some("https://github.com/search?type=code&q=x".into())
        );
        assert_eq!(table.get("yt"), None);
        assert_eq!(table.len(), DEFAULTS.len());
    }

    #[test]
    fn defaults_are_valid_and_sorted() {
        let mut table = BangTable::empty();
        for &(trigger, _, template) in DEFAULTS {
            table.insert(trigger, template).unwrap();
        }
        let triggers: Vec<_> = DEFAULTS.iter().map(|d| d.0).collect();
        let mut sorted = triggers.clone();
        sorted.sort_unstable();
        assert_eq!(triggers, sorted);
        assert_eq!(table.len(), DEFAULTS.len());
    }

    #[test]
    fn plugs_into_the_omnibox() {
        let omnibox = Omnibox::default().with_step(Arc::new(BangTable::default()));
        assert_eq!(
            omnibox.resolve("!w Qt"),
            Some(Resolved::Expanded(
                "https://en.wikipedia.org/w/index.php?search=Qt".into()
            ))
        );
        assert!(matches!(
            omnibox.resolve("!unknown thing"),
            Some(Resolved::Search(_))
        ));
        assert!(matches!(
            omnibox.resolve("example.com"),
            Some(Resolved::Url(_))
        ));
    }
}
