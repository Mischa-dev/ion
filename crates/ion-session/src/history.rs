//! Browsing history: the pages people visited, searchable by URL and title.
//!
//! Kept small on purpose: one JSON file, capped at [`MAX_ENTRIES`] pages, ranked
//! by "frecency" (how often and how recently a page was visited). The command
//! palette searches it; nothing else reads it yet.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::paths::write_atomic;

/// Pages kept before the least valuable ones are dropped.
pub const MAX_ENTRIES: usize = 5000;

/// Pruning drops down to this many pages, so it runs once per few hundred
/// new pages rather than on every visit once history is full.
const PRUNED_ENTRIES: usize = MAX_ENTRIES * 9 / 10;

const DAY_SECS: u64 = 24 * 60 * 60;

/// One visited page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub url: String,
    #[serde(default)]
    pub title: String,
    pub visits: u32,
    /// Unix time (seconds) of the latest visit.
    pub last_visit: u64,
}

impl HistoryEntry {
    /// Visit count weighted by how recently the page was last visited.
    pub fn frecency(&self, now: u64) -> f64 {
        let age_days = now.saturating_sub(self.last_visit) / DAY_SECS;
        let recency = match age_days {
            0..=3 => 1.0,
            4..=14 => 0.7,
            15..=31 => 0.5,
            32..=90 => 0.3,
            _ => 0.1,
        };
        f64::from(self.visits) * recency
    }
}

#[derive(Default, Serialize, Deserialize)]
struct HistoryFile {
    #[serde(default)]
    entries: Vec<HistoryEntry>,
}

/// The visited pages, newest knowledge first.
#[derive(Debug, Default)]
pub struct History {
    entries: Vec<HistoryEntry>,
    by_url: HashMap<String, usize>,
}

/// Whether a URL belongs in history. Internal and blank pages do not.
pub fn is_recordable(url: &str) -> bool {
    ["https://", "http://", "file://"]
        .iter()
        .any(|scheme| url.starts_with(scheme))
}

/// Seconds since the Unix epoch.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    /// `<data dir>/history.json`.
    pub fn default_path() -> Option<PathBuf> {
        crate::paths::data_dir().map(|d| d.join("history.json"))
    }

    /// Load history from `path`; a missing file is an empty history.
    pub fn load(path: &Path) -> io::Result<Self> {
        let file: HistoryFile = match std::fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => HistoryFile::default(),
            Err(e) => return Err(e),
        };
        let mut history = Self {
            entries: file.entries,
            by_url: HashMap::new(),
        };
        history.reindex();
        Ok(history)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let file = HistoryFile {
            entries: self.entries.clone(),
        };
        let json = serde_json::to_vec(&file).map_err(io::Error::other)?;
        write_atomic(path, &json)
    }

    fn reindex(&mut self) {
        self.by_url = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.url.clone(), i))
            .collect();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, url: &str) -> Option<&HistoryEntry> {
        self.by_url.get(url).map(|&i| &self.entries[i])
    }

    /// Record a visit to `url` at time `now`. Returns false (and records
    /// nothing) for URLs that do not belong in history.
    pub fn record_visit(&mut self, url: &str, title: &str, now: u64) -> bool {
        if !is_recordable(url) {
            return false;
        }
        match self.by_url.get(url) {
            Some(&i) => {
                let entry = &mut self.entries[i];
                entry.visits = entry.visits.saturating_add(1);
                entry.last_visit = entry.last_visit.max(now);
                if !title.is_empty() {
                    entry.title = title.to_owned();
                }
            }
            None => {
                self.by_url.insert(url.to_owned(), self.entries.len());
                self.entries.push(HistoryEntry {
                    url: url.to_owned(),
                    title: title.to_owned(),
                    visits: 1,
                    last_visit: now,
                });
                if self.entries.len() > MAX_ENTRIES {
                    self.prune(now);
                }
            }
        }
        true
    }

    /// Merge pages from another browser's history. A page already here keeps
    /// the larger visit count, the later visit and its title unless it has
    /// none. Returns how many new pages were added.
    pub fn merge(&mut self, entries: impl IntoIterator<Item = HistoryEntry>, now: u64) -> usize {
        let mut added = 0;
        for entry in entries {
            if !is_recordable(&entry.url) {
                continue;
            }
            match self.by_url.get(&entry.url) {
                Some(&i) => {
                    let here = &mut self.entries[i];
                    here.visits = here.visits.max(entry.visits);
                    here.last_visit = here.last_visit.max(entry.last_visit);
                    if here.title.is_empty() {
                        here.title = entry.title;
                    }
                }
                None => {
                    self.by_url.insert(entry.url.clone(), self.entries.len());
                    self.entries.push(entry);
                    added += 1;
                }
            }
        }
        if self.entries.len() > MAX_ENTRIES {
            self.prune(now);
        }
        added
    }

    /// Update the title of a page already in history. True if it changed.
    pub fn set_title(&mut self, url: &str, title: &str) -> bool {
        match self.by_url.get(url) {
            Some(&i) if !title.is_empty() && self.entries[i].title != title => {
                self.entries[i].title = title.to_owned();
                true
            }
            _ => false,
        }
    }

    /// Forget one page. True if it was there.
    pub fn remove(&mut self, url: &str) -> bool {
        if self.by_url.remove(url).is_none() {
            return false;
        }
        self.entries.retain(|e| e.url != url);
        self.reindex();
        true
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.by_url.clear();
    }

    /// Drop the least valuable pages, keeping [`PRUNED_ENTRIES`].
    fn prune(&mut self, now: u64) {
        self.entries.sort_by(|a, b| {
            b.frecency(now)
                .total_cmp(&a.frecency(now))
                .then(b.last_visit.cmp(&a.last_visit))
        });
        self.entries.truncate(PRUNED_ENTRIES);
        self.reindex();
    }

    /// Pages matching `query`, best first, at most `limit`.
    ///
    /// Every whitespace-separated word of the query has to appear in the URL or
    /// the title (case-insensitive). Pages whose host starts with the first word
    /// rank higher, so "git" finds github.com before a page that mentions git.
    /// An empty query returns the most frecent pages.
    pub fn search(&self, query: &str, limit: usize, now: u64) -> Vec<&HistoryEntry> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let mut hits: Vec<(f64, &HistoryEntry)> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let url = entry.url.to_lowercase();
                let title = entry.title.to_lowercase();
                if !words
                    .iter()
                    .all(|w| url.contains(w.as_str()) || title.contains(w.as_str()))
                {
                    return None;
                }
                let host_bonus = match words.first() {
                    Some(first) if host(&url).starts_with(first.as_str()) => 4.0,
                    _ => 1.0,
                };
                Some((entry.frecency(now) * host_bonus, entry))
            })
            .collect();
        hits.sort_by(|a, b| {
            b.0.total_cmp(&a.0)
                .then(b.1.last_visit.cmp(&a.1.last_visit))
        });
        hits.into_iter().take(limit).map(|(_, e)| e).collect()
    }
}

/// The host of a lowercase URL without a leading `www.`.
fn host(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    host.strip_prefix("www.").unwrap_or(host)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_dir;

    const NOW: u64 = 1_800_000_000;

    fn urls<'a>(entries: &[&'a HistoryEntry]) -> Vec<&'a str> {
        entries.iter().map(|e| e.url.as_str()).collect()
    }

    #[test]
    fn records_web_and_file_pages_only() {
        let mut h = History::new();
        assert!(h.record_visit("https://example.com/", "Example", NOW));
        assert!(h.record_visit("file:///tmp/a.html", "", NOW));
        assert!(!h.record_visit("about:blank", "", NOW));
        assert!(!h.record_visit("chrome://gpu", "", NOW));
        assert!(!h.record_visit("", "", NOW));
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn repeat_visits_count_and_keep_titles() {
        let mut h = History::new();
        h.record_visit("https://a.example/", "A", NOW);
        h.record_visit("https://a.example/", "", NOW + 10);
        let e = h.get("https://a.example/").unwrap();
        assert_eq!(
            (e.visits, e.last_visit, e.title.as_str()),
            (2, NOW + 10, "A")
        );
        assert!(h.set_title("https://a.example/", "A2"));
        assert!(!h.set_title("https://a.example/", "A2"));
        assert!(!h.set_title("https://a.example/", ""));
        assert!(!h.set_title("https://missing.example/", "x"));
    }

    #[test]
    fn search_matches_all_words_in_url_or_title() {
        let mut h = History::new();
        h.record_visit("https://docs.rs/serde", "serde - Rust", NOW);
        h.record_visit("https://doc.qt.io/qt-6/qml.html", "QML Reference", NOW);
        h.record_visit("https://example.com/", "Example Domain", NOW);
        assert_eq!(
            urls(&h.search("RUST serde", 10, NOW)),
            ["https://docs.rs/serde"]
        );
        assert_eq!(
            urls(&h.search("qml reference", 10, NOW)),
            ["https://doc.qt.io/qt-6/qml.html"]
        );
        assert!(h.search("nothing here", 10, NOW).is_empty());
    }

    #[test]
    fn host_prefix_and_frecency_rank_results() {
        let mut h = History::new();
        for _ in 0..3 {
            h.record_visit("https://blog.example/why-git-is-hard", "Git post", NOW);
        }
        h.record_visit("https://github.com/", "GitHub", NOW);
        h.record_visit("https://www.gitlab.com/", "GitLab", NOW - 60 * DAY_SECS);
        assert_eq!(
            urls(&h.search("git", 10, NOW)),
            [
                "https://github.com/",
                "https://blog.example/why-git-is-hard",
                "https://www.gitlab.com/"
            ]
        );
    }

    #[test]
    fn empty_query_lists_top_pages_with_limit() {
        let mut h = History::new();
        h.record_visit("https://a.example/", "", NOW);
        h.record_visit("https://b.example/", "", NOW);
        h.record_visit("https://b.example/", "", NOW);
        assert_eq!(urls(&h.search("", 1, NOW)), ["https://b.example/"]);
    }

    #[test]
    fn merge_adds_new_pages_and_keeps_the_best_of_known_ones() {
        let mut h = History::new();
        h.record_visit("https://a.example/", "", NOW);
        let entry = |url: &str, title: &str, visits, last_visit| HistoryEntry {
            url: url.into(),
            title: title.into(),
            visits,
            last_visit,
        };
        let added = h.merge(
            [
                entry("https://a.example/", "A", 7, NOW - 100),
                entry("https://b.example/", "B", 2, NOW - 50),
                entry("about:blank", "", 1, NOW),
            ],
            NOW,
        );
        assert_eq!(added, 1);
        let a = h.get("https://a.example/").unwrap();
        assert_eq!((a.title.as_str(), a.visits, a.last_visit), ("A", 7, NOW));
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn remove_and_clear() {
        let mut h = History::new();
        h.record_visit("https://a.example/", "", NOW);
        h.record_visit("https://b.example/", "", NOW);
        assert!(h.remove("https://a.example/"));
        assert!(!h.remove("https://a.example/"));
        assert_eq!(h.get("https://b.example/").unwrap().visits, 1);
        h.clear();
        assert!(h.is_empty());
    }

    #[test]
    fn prunes_least_valuable_pages() {
        let mut h = History::new();
        h.record_visit("https://keep.example/", "", NOW);
        h.record_visit("https://keep.example/", "", NOW);
        h.record_visit("https://old.example/", "", NOW - 365 * DAY_SECS);
        for i in 0..MAX_ENTRIES - 1 {
            h.record_visit(&format!("https://p{i}.example/"), "", NOW);
        }
        assert_eq!(h.len(), PRUNED_ENTRIES);
        assert!(h.get("https://old.example/").is_none());
        assert_eq!(h.get("https://keep.example/").unwrap().visits, 2);
        h.record_visit("https://keep.example/", "", NOW);
        assert_eq!(h.get("https://keep.example/").unwrap().visits, 3);
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = test_dir("history");
        let path = dir.join("history.json");
        assert!(History::load(&path).unwrap().is_empty());
        let mut h = History::new();
        h.record_visit("https://a.example/", "A", NOW);
        h.save(&path).unwrap();
        let loaded = History::load(&path).unwrap();
        assert_eq!(
            loaded.get("https://a.example/"),
            h.get("https://a.example/")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn host_strips_scheme_www_and_path() {
        assert_eq!(host("https://www.github.com/a?b"), "github.com");
        assert_eq!(host("file:///tmp/x"), "");
        assert_eq!(host("example.com#x"), "example.com");
    }
}
