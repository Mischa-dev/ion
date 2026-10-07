//! The bookmark list.
//!
//! A flat list in the order bookmarks were added. Folders are a `/`-separated
//! path on each bookmark ("Imported from Firefox/Toolbar"), which keeps the
//! file simple and is enough for search; there is no folder tree to manage.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One bookmarked page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub url: String,
    #[serde(default)]
    pub title: String,
    /// `/`-separated folder path, empty for top level.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub folder: String,
    /// Unix time (seconds) the bookmark was added.
    #[serde(default)]
    pub added: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct BookmarksFile {
    #[serde(default)]
    bookmarks: Vec<Bookmark>,
}

/// All bookmarks. Each URL is bookmarked at most once.
#[derive(Debug, Default)]
pub struct Bookmarks {
    items: Vec<Bookmark>,
}

/// Whether a URL can be bookmarked. Internal and blank pages can not.
pub fn is_bookmarkable(url: &str) -> bool {
    ["https://", "http://", "file://"]
        .iter()
        .any(|scheme| url.starts_with(scheme))
}

impl Bookmarks {
    pub fn new() -> Self {
        Self::default()
    }

    /// `<data dir>/bookmarks.json`.
    pub fn default_path() -> Option<PathBuf> {
        ion_session::paths::data_dir().map(|d| d.join("bookmarks.json"))
    }

    /// Load bookmarks from `path`; a missing file means no bookmarks.
    pub fn load(path: &Path) -> io::Result<Self> {
        let file: BookmarksFile = match std::fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => BookmarksFile::default(),
            Err(e) => return Err(e),
        };
        let mut bookmarks = Self::new();
        for b in file.bookmarks {
            bookmarks.insert(b);
        }
        Ok(bookmarks)
    }

    /// Write bookmarks to `path` atomically.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let file = BookmarksFile {
            bookmarks: self.items.clone(),
        };
        let json = serde_json::to_vec_pretty(&file).map_err(io::Error::other)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, path)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Bookmark> {
        self.items.iter()
    }

    pub fn contains(&self, url: &str) -> bool {
        self.items.iter().any(|b| b.url == url)
    }

    /// Add a bookmark. False (and nothing changes) if the URL is already
    /// bookmarked or can't be.
    pub fn insert(&mut self, bookmark: Bookmark) -> bool {
        if !is_bookmarkable(&bookmark.url) || self.contains(&bookmark.url) {
            return false;
        }
        self.items.push(bookmark);
        true
    }

    /// Add many bookmarks, skipping URLs already present. Returns how many
    /// were added.
    pub fn extend(&mut self, bookmarks: impl IntoIterator<Item = Bookmark>) -> usize {
        let mut seen: HashSet<String> = self.items.iter().map(|b| b.url.clone()).collect();
        let before = self.items.len();
        for b in bookmarks {
            if is_bookmarkable(&b.url) && seen.insert(b.url.clone()) {
                self.items.push(b);
            }
        }
        self.items.len() - before
    }

    /// Remove the bookmark for `url`. True if there was one.
    pub fn remove(&mut self, url: &str) -> bool {
        let before = self.items.len();
        self.items.retain(|b| b.url != url);
        self.items.len() != before
    }

    /// Bookmark `url` if it isn't yet, otherwise remove it. Returns whether
    /// the page is bookmarked afterwards.
    pub fn toggle(&mut self, url: &str, title: &str, now: u64) -> bool {
        if self.remove(url) {
            return false;
        }
        self.insert(Bookmark {
            url: url.to_owned(),
            title: title.to_owned(),
            folder: String::new(),
            added: now,
        })
    }

    /// Bookmarks matching `query`, best first, at most `limit`.
    ///
    /// Every whitespace-separated word has to appear in the URL, title or
    /// folder (case-insensitive). Title matches rank above URL-only matches,
    /// and newer bookmarks above older ones. An empty query lists the newest.
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Bookmark> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let mut hits: Vec<(u32, &Bookmark)> = self
            .items
            .iter()
            .filter_map(|b| {
                let url = b.url.to_lowercase();
                let title = b.title.to_lowercase();
                let folder = b.folder.to_lowercase();
                let mut score = 0;
                for w in &words {
                    if title.contains(w.as_str()) {
                        score += 3;
                    } else if url.contains(w.as_str()) {
                        score += 2;
                    } else if folder.contains(w.as_str()) {
                        score += 1;
                    } else {
                        return None;
                    }
                }
                Some((score, b))
            })
            .collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.added.cmp(&a.1.added)));
        hits.into_iter().take(limit).map(|(_, b)| b).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bm(url: &str, title: &str, added: u64) -> Bookmark {
        Bookmark {
            url: url.into(),
            title: title.into(),
            folder: String::new(),
            added,
        }
    }

    #[test]
    fn toggle_adds_then_removes() {
        let mut b = Bookmarks::new();
        assert!(b.toggle("https://example.com/", "Example", 1));
        assert!(b.contains("https://example.com/"));
        assert!(!b.toggle("https://example.com/", "Example", 2));
        assert!(b.is_empty());
    }

    #[test]
    fn internal_pages_are_not_bookmarked() {
        let mut b = Bookmarks::new();
        assert!(!b.toggle("about:blank", "", 1));
        assert!(!b.insert(bm("chrome://gpu", "", 1)));
        assert!(b.is_empty());
    }

    #[test]
    fn extend_skips_duplicates() {
        let mut b = Bookmarks::new();
        b.insert(bm("https://a.example/", "A", 1));
        let added = b.extend([
            bm("https://a.example/", "A again", 2),
            bm("https://b.example/", "B", 2),
            bm("https://b.example/", "B twice", 3),
        ]);
        assert_eq!(added, 1);
        assert_eq!(b.len(), 2);
    }

    #[test]
    fn search_needs_every_word_and_ranks_titles_first() {
        let mut b = Bookmarks::new();
        b.insert(bm("https://docs.rs/rust", "Docs", 1));
        b.insert(bm("https://example.com/", "Rust book", 2));
        b.insert(bm("https://nixos.org/", "NixOS", 3));
        let urls: Vec<_> = b.search("rust", 10).iter().map(|b| b.url.clone()).collect();
        assert_eq!(urls, ["https://example.com/", "https://docs.rs/rust"]);
        assert!(b.search("rust nix", 10).is_empty());
        assert_eq!(b.search("", 10)[0].url, "https://nixos.org/");
        assert_eq!(b.search("", 2).len(), 2);
    }

    #[test]
    fn search_matches_folders() {
        let mut b = Bookmarks::new();
        let mut x = bm("https://x.example/", "X", 1);
        x.folder = "Imported from Firefox/Toolbar".into();
        b.insert(x);
        assert_eq!(b.search("firefox", 10).len(), 1);
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = std::env::temp_dir().join(format!("ion-bookmarks-{}", std::process::id()));
        let path = dir.join("bookmarks.json");
        let mut b = Bookmarks::new();
        b.insert(bm("https://example.com/", "Example", 5));
        b.save(&path).unwrap();
        let loaded = Bookmarks::load(&path).unwrap();
        assert_eq!(
            loaded.iter().collect::<Vec<_>>(),
            b.iter().collect::<Vec<_>>()
        );
        assert!(
            Bookmarks::load(&dir.join("missing.json"))
                .unwrap()
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
