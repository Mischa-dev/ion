//! Conversations kept across restarts, so the person can reopen one from the
//! agent sidebar and carry on: each question, the steps the agent took and
//! its answer, in `conversations.json`.
//!
//! The file is private to the user (0600 in a 0700 directory on Unix). Page
//! contents, tool results and selections are never kept. A reopened
//! conversation continues from the questions and answers alone (see
//! [`crate::Task::reopen`]).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const VERSION: u32 = 1;

/// The most conversations kept, whatever their age.
pub const MAX_CONVERSATIONS: usize = 200;

const DAY: u64 = 24 * 60 * 60;

/// One line of a saved exchange's log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedStep {
    pub text: String,
    /// A [`crate::StepStatus`] id.
    pub status: String,
}

/// A saved question and how it went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedExchange {
    pub question: String,
    #[serde(default)]
    pub steps: Vec<SavedStep>,
    #[serde(default)]
    pub answer: String,
    #[serde(default)]
    pub error: String,
    /// A [`crate::Status`] id.
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    /// Unique among saved conversations.
    pub key: String,
    pub agent: String,
    /// Unix seconds.
    pub started: u64,
    pub updated: u64,
    /// The page the first question was asked from.
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub title: String,
    pub exchanges: Vec<SavedExchange>,
}

#[derive(Serialize, Deserialize)]
struct File {
    version: u32,
    conversations: Vec<Conversation>,
}

/// The saved conversations, newest first, and where they live.
#[derive(Debug, Default)]
pub struct History {
    path: Option<PathBuf>,
    keep_days: u32,
    conversations: Vec<Conversation>,
}

impl History {
    /// Kept in memory only: nothing is read or written.
    pub fn in_memory(keep_days: u32) -> History {
        History {
            path: None,
            keep_days,
            conversations: Vec::new(),
        }
    }

    /// Read `path`, keeping conversations updated in the last `keep_days`
    /// days (none when 0). A missing or unreadable file is no
    /// conversations; an unreadable one is moved aside, not overwritten.
    pub fn open(path: impl Into<PathBuf>, keep_days: u32, now: u64) -> History {
        let path = path.into();
        let conversations = match read(&path) {
            Ok(conversations) => conversations,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(_) => {
                let mut aside = path.as_os_str().to_owned();
                aside.push(".unreadable");
                let _ = fs::rename(&path, PathBuf::from(aside));
                Vec::new()
            }
        };
        let mut history = History {
            path: Some(path),
            keep_days,
            conversations,
        };
        if history.prune(now) {
            history.write();
        }
        history
    }

    /// Newest first.
    pub fn list(&self) -> &[Conversation] {
        &self.conversations
    }

    pub fn get(&self, key: &str) -> Option<&Conversation> {
        self.conversations.iter().find(|c| c.key == key)
    }

    /// Save `conversation`, replacing the one with its key, as the newest.
    /// Nothing is kept while `keep_days` is 0.
    pub fn put(&mut self, conversation: Conversation, now: u64) {
        self.conversations.retain(|c| c.key != conversation.key);
        if self.keep_days > 0 {
            self.conversations.insert(0, conversation);
        }
        self.prune(now);
        self.write();
    }

    /// Forget one conversation. True if it was there.
    pub fn forget(&mut self, key: &str) -> bool {
        let before = self.conversations.len();
        self.conversations.retain(|c| c.key != key);
        let forgot = self.conversations.len() != before;
        if forgot {
            self.write();
        }
        forgot
    }

    pub fn forget_all(&mut self) {
        self.conversations.clear();
        self.write();
    }

    /// Follow a config change; 0 forgets everything.
    pub fn set_keep_days(&mut self, keep_days: u32, now: u64) {
        if keep_days == self.keep_days {
            return;
        }
        self.keep_days = keep_days;
        if self.prune(now) {
            self.write();
        }
    }

    /// Drop what is too old or too many. True if anything went.
    fn prune(&mut self, now: u64) -> bool {
        let before = self.conversations.len();
        let oldest = now.saturating_sub(u64::from(self.keep_days) * DAY);
        let keep_days = self.keep_days;
        self.conversations
            .retain(|c| keep_days > 0 && c.updated >= oldest);
        self.conversations
            .sort_by_key(|c| std::cmp::Reverse(c.updated));
        self.conversations.truncate(MAX_CONVERSATIONS);
        self.conversations.len() != before
    }

    /// Saving is best effort: a full disk loses history, not the browser.
    fn write(&self) {
        let Some(path) = &self.path else {
            return;
        };
        let file = File {
            version: VERSION,
            conversations: self.conversations.clone(),
        };
        if let Ok(json) = serde_json::to_vec(&file) {
            let _ = write_atomic(path, &json);
        }
    }
}

fn read(path: &Path) -> io::Result<Vec<Conversation>> {
    let text = fs::read_to_string(path)?;
    let file: File = serde_json::from_str(&text).map_err(io::Error::other)?;
    if file.version > VERSION {
        return Err(io::Error::other(
            "conversations file is newer than this Ion",
        ));
    }
    Ok(file.conversations)
}

fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.create(parent)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    io::Write::write_all(&mut options.open(&tmp)?, contents)?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ion-agent-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn conversation(key: &str, updated: u64) -> Conversation {
        Conversation {
            key: key.into(),
            agent: "ion".into(),
            started: updated,
            updated,
            url: "https://example.com/".into(),
            title: "Example".into(),
            exchanges: vec![SavedExchange {
                question: "what is this?".into(),
                steps: vec![SavedStep {
                    text: "Read example.com".into(),
                    status: "done".into(),
                }],
                answer: "An example.".into(),
                error: String::new(),
                status: "done".into(),
            }],
        }
    }

    #[test]
    fn survives_a_restart_newest_first() {
        let dir = dir("history-restart");
        let path = dir.join("agent/conversations.json");
        let now = 100 * DAY;
        let mut history = History::open(&path, 30, now);
        history.put(conversation("a", now - 10), now);
        history.put(conversation("b", now - 5), now);
        // Updating moves it to the front.
        history.put(conversation("a", now), now);

        let reopened = History::open(&path, 30, now);
        let keys: Vec<&str> = reopened.list().iter().map(|c| c.key.as_str()).collect();
        assert_eq!(keys, ["a", "b"]);
        assert_eq!(reopened.get("b"), Some(&conversation("b", now - 5)));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn old_ones_go_and_zero_days_keeps_none() {
        let now = 100 * DAY;
        let mut history = History::in_memory(30);
        history.put(conversation("old", now - 31 * DAY), now);
        history.put(conversation("new", now - DAY), now);
        assert_eq!(history.list().len(), 1);
        assert_eq!(history.list()[0].key, "new");

        history.set_keep_days(0, now);
        assert!(history.list().is_empty());
        history.put(conversation("later", now), now);
        assert!(history.list().is_empty());
    }

    #[test]
    fn keeps_at_most_the_newest() {
        let now = 100 * DAY;
        let mut history = History::in_memory(30);
        for i in 0..MAX_CONVERSATIONS as u64 + 5 {
            history.put(conversation(&i.to_string(), now - 1000 + i), now);
        }
        assert_eq!(history.list().len(), MAX_CONVERSATIONS);
        assert_eq!(history.list()[0].key, (MAX_CONVERSATIONS + 4).to_string());
    }

    #[test]
    fn forgetting_writes_through() {
        let dir = dir("history-forget");
        let path = dir.join("conversations.json");
        let mut history = History::open(&path, 30, 10);
        history.put(conversation("a", 10), 10);
        history.put(conversation("b", 10), 10);
        assert!(history.forget("a"));
        assert!(!history.forget("a"));
        assert_eq!(History::open(&path, 30, 10).list().len(), 1);
        history.forget_all();
        assert!(History::open(&path, 30, 10).list().is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unreadable_file_is_set_aside() {
        let dir = dir("history-unreadable");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("conversations.json");
        fs::write(&path, "not json").unwrap();
        let history = History::open(&path, 30, 10);
        assert!(history.list().is_empty());
        assert!(dir.join("conversations.json.unreadable").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
