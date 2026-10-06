//! Saved sessions: the tabs to reopen at startup, and named sessions.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::paths::write_atomic;

/// Format version written into every session file.
pub const SESSION_VERSION: u32 = 1;

/// A tab as stored on disk.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedTab {
    pub url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
}

/// A snapshot of one window's tabs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub tabs: Vec<SavedTab>,
    /// Index of the current tab.
    #[serde(default)]
    pub current: usize,
    /// Tab strip shown as a vertical sidebar. Only read from the last session.
    #[serde(default)]
    pub vertical_tabs: bool,
}

fn default_version() -> u32 {
    SESSION_VERSION
}

impl Default for Session {
    fn default() -> Self {
        Self {
            version: SESSION_VERSION,
            tabs: Vec::new(),
            current: 0,
            vertical_tabs: false,
        }
    }
}

impl Session {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a session always serializes")
    }

    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }
}

/// Session files under one directory:
///
/// ```text
/// <dir>/last.json            the tabs open when Ion last ran
/// <dir>/named/<name>.json    sessions saved by name
/// ```
#[derive(Clone, Debug)]
pub struct SessionStore {
    dir: PathBuf,
}

impl SessionStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The store in Ion's data directory (`<data dir>/sessions`).
    pub fn in_data_dir() -> Option<Self> {
        crate::paths::data_dir().map(|d| Self::new(d.join("sessions")))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn last_path(&self) -> PathBuf {
        self.dir.join("last.json")
    }

    fn named_dir(&self) -> PathBuf {
        self.dir.join("named")
    }

    fn named_path(&self, name: &str) -> PathBuf {
        self.named_dir()
            .join(format!("{}.json", encode_name(name.trim())))
    }

    /// The session Ion should restore. `Ok(None)` when there is none yet.
    pub fn load_last(&self) -> io::Result<Option<Session>> {
        read(&self.last_path())
    }

    pub fn save_last(&self, session: &Session) -> io::Result<()> {
        write_atomic(&self.last_path(), session.to_json().as_bytes())
    }

    /// Save `session` under `name`, replacing any session with that name.
    pub fn save_named(&self, name: &str, session: &Session) -> io::Result<()> {
        if name.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "session name is empty",
            ));
        }
        write_atomic(&self.named_path(name), session.to_json().as_bytes())
    }

    pub fn load_named(&self, name: &str) -> io::Result<Option<Session>> {
        read(&self.named_path(name))
    }

    /// Delete a named session. True if it existed.
    pub fn delete_named(&self, name: &str) -> io::Result<bool> {
        match std::fs::remove_file(self.named_path(name)) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Names of the saved sessions, sorted case-insensitively.
    pub fn list_named(&self) -> io::Result<Vec<String>> {
        let entries = match std::fs::read_dir(self.named_dir()) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|entry| {
                let file = entry.file_name();
                let stem = file.to_str()?.strip_suffix(".json")?;
                decode_name(stem)
            })
            .collect();
        names.sort_by_key(|n| n.to_lowercase());
        Ok(names)
    }
}

fn read(path: &Path) -> io::Result<Option<Session>> {
    match std::fs::read_to_string(path) {
        Ok(json) => Session::from_json(&json)
            .map(Some)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Encode a session name as a file name: ASCII letters, digits, `-` and `_`
/// stay as they are, every other byte becomes `%XX`.
fn encode_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn decode_name(stem: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(stem.len());
    let mut iter = stem.bytes();
    while let Some(byte) = iter.next() {
        if byte == b'%' {
            let hex = [iter.next()?, iter.next()?];
            bytes.push(u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?);
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::test_dir;

    fn session(urls: &[&str]) -> Session {
        Session {
            tabs: urls
                .iter()
                .map(|u| SavedTab {
                    url: u.to_string(),
                    title: String::new(),
                })
                .collect(),
            ..Session::default()
        }
    }

    #[test]
    fn last_session_round_trip() {
        let dir = test_dir("last");
        let store = SessionStore::new(&dir);
        assert_eq!(store.load_last().unwrap(), None);
        let mut s = session(&["https://a.example/", "https://b.example/"]);
        s.current = 1;
        s.vertical_tabs = true;
        store.save_last(&s).unwrap();
        assert_eq!(store.load_last().unwrap(), Some(s));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn named_sessions_list_load_delete() {
        let dir = test_dir("named");
        let store = SessionStore::new(&dir);
        assert!(store.list_named().unwrap().is_empty());
        store.save_named("work", &session(&["w"])).unwrap();
        store
            .save_named("Reading list / 2026", &session(&["r"]))
            .unwrap();
        store.save_named("Ünïcode ✓", &session(&["u"])).unwrap();
        assert_eq!(
            store.list_named().unwrap(),
            ["Reading list / 2026", "work", "Ünïcode ✓"]
        );
        assert_eq!(
            store.load_named("Reading list / 2026").unwrap(),
            Some(session(&["r"]))
        );
        assert!(store.delete_named("work").unwrap());
        assert!(!store.delete_named("work").unwrap());
        assert_eq!(store.load_named("work").unwrap(), None);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn saving_a_name_again_replaces_it() {
        let dir = test_dir("replace");
        let store = SessionStore::new(&dir);
        store.save_named("x", &session(&["1"])).unwrap();
        store.save_named(" x ", &session(&["2"])).unwrap();
        assert_eq!(store.list_named().unwrap(), ["x"]);
        assert_eq!(store.load_named("x").unwrap(), Some(session(&["2"])));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn empty_name_is_rejected() {
        let store = SessionStore::new(test_dir("empty"));
        assert!(store.save_named("  ", &session(&[])).is_err());
    }

    #[test]
    fn corrupt_file_is_an_error_not_a_panic() {
        let dir = test_dir("corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("last.json"), "{ nope").unwrap();
        let err = SessionStore::new(&dir).load_last().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_fields_use_defaults() {
        let s = Session::from_json(r#"{"tabs":[{"url":"a"}]}"#).unwrap();
        assert_eq!(s.version, SESSION_VERSION);
        assert_eq!(s.current, 0);
        assert!(!s.vertical_tabs);
        assert_eq!(s.tabs[0].title, "");
    }

    #[test]
    fn name_encoding_round_trips() {
        for name in [
            "plain",
            "with space",
            "a/b\\c",
            "100%",
            "émoji 🚀",
            "dots..",
        ] {
            let encoded = encode_name(name);
            assert!(
                encoded
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_%".contains(&b))
            );
            assert_eq!(decode_name(&encoded).as_deref(), Some(name));
        }
        assert_eq!(decode_name("bad%2"), None);
    }
}
