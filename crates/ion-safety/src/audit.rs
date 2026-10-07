//! The activity log: an append-only record of what agents asked for, what Ion
//! decided, what agents did, and every permission answer or rule change.
//!
//! One JSON Lines file per UTC day (`YYYY-MM-DD.jsonl`). Entries hold no page
//! content; URLs lose their query and fragment, and sensitive text is
//! replaced by its length.

use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::capability::{Action, SiteCapability};
use crate::policy::Verdict;

/// Longest detail kept, in characters.
pub const MAX_DETAIL_CHARS: usize = 500;

/// Days of log kept by default.
pub const DEFAULT_RETENTION_DAYS: u32 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    /// Ion decided on a request (allow, deny, or ask).
    Decision,
    /// The person answered a prompt.
    Answer,
    /// An agent did something (or tried and failed).
    Action,
    /// The person removed a remembered rule.
    Forget,
    /// The person stopped all agents.
    Stop,
    /// The person let agents run again.
    Resume,
    /// The person took a tab back from agents.
    TakeOver,
    /// The person handed a tab back to agents.
    HandBack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// Unix seconds.
    pub time: u64,
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<SiteCapability>,
    /// Origin, or connector name for connector actions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab: Option<u64>,
    /// The agent's task this belongs to, when the runtime has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
    /// Why, in words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// For actions: whether it worked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    /// A short description, already redacted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Entry {
    pub fn new(time: u64, kind: Kind) -> Entry {
        Entry {
            time,
            kind,
            agent: None,
            action: None,
            capability: None,
            target: None,
            tab: None,
            task: None,
            verdict: None,
            reason: None,
            ok: None,
            detail: None,
        }
    }
}

/// Text for the log: kept (and shortened) when ordinary, replaced by its
/// length when `sensitive` (password and personal-data fields).
pub fn redact(text: &str, sensitive: bool) -> String {
    let chars = text.chars().count();
    if sensitive {
        return format!("[redacted, {chars} chars]");
    }
    if chars <= MAX_DETAIL_CHARS {
        return text.to_owned();
    }
    let mut short: String = text.chars().take(MAX_DETAIL_CHARS).collect();
    short.push('…');
    short
}

/// The UTC date of `unix_seconds` as `YYYY-MM-DD`.
pub fn utc_date(unix_seconds: u64) -> String {
    // Howard Hinnant's days-to-civil algorithm.
    let days = (unix_seconds / 86_400) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// The log's directory and how long it keeps files.
#[derive(Debug, Clone)]
pub struct AuditLog {
    dir: PathBuf,
    retention_days: u32,
}

impl AuditLog {
    pub fn new(dir: impl Into<PathBuf>) -> AuditLog {
        AuditLog {
            dir: dir.into(),
            retention_days: DEFAULT_RETENTION_DAYS,
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// At least one day is always kept.
    pub fn set_retention_days(&mut self, days: u32) {
        self.retention_days = days.max(1);
    }

    fn file_for(&self, time: u64) -> PathBuf {
        self.dir.join(format!("{}.jsonl", utc_date(time)))
    }

    pub fn append(&self, entry: &Entry) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let mut line = serde_json::to_string(entry).map_err(io::Error::other)?;
        line.push('\n');
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.file_for(entry.time))?
            .write_all(line.as_bytes())
    }

    /// Log files, oldest first, as (date, path).
    fn files(&self) -> io::Result<Vec<(String, PathBuf)>> {
        let mut files = Vec::new();
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(files),
            Err(e) => return Err(e),
        };
        for entry in entries {
            let path = entry?.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            match name.strip_suffix(".jsonl") {
                Some(date) if date.len() == 10 => files.push((date.to_owned(), path)),
                _ => {}
            }
        }
        files.sort();
        Ok(files)
    }

    /// Entries at or after `since` (unix seconds), oldest first. Lines that
    /// don't parse (a torn write after a crash) are skipped.
    pub fn read_since(&self, since: u64) -> io::Result<Vec<Entry>> {
        let first_day = utc_date(since);
        let mut out = Vec::new();
        for (date, path) in self.files()? {
            if date < first_day {
                continue;
            }
            for line in BufReader::new(fs::File::open(path)?).lines() {
                match serde_json::from_str::<Entry>(&line?) {
                    Ok(entry) if entry.time >= since => out.push(entry),
                    _ => {}
                }
            }
        }
        Ok(out)
    }

    /// Delete files older than the retention period. Returns how many went.
    pub fn prune(&self, now: u64) -> io::Result<usize> {
        let keep_from = utc_date(now.saturating_sub(u64::from(self.retention_days - 1) * 86_400));
        let mut removed = 0;
        for (date, path) in self.files()? {
            if date < keep_from {
                fs::remove_file(path)?;
                removed += 1;
            }
        }
        Ok(removed)
    }
}

#[cfg(test)]
pub(crate) fn test_dir(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ion-safety-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 86_400;
    // 2026-10-07T10:00:00Z
    const NOW: u64 = 1_791_367_200;

    #[test]
    fn dates() {
        assert_eq!(utc_date(0), "1970-01-01");
        assert_eq!(utc_date(951_782_400), "2000-02-29");
        assert_eq!(utc_date(NOW), "2026-10-07");
        assert_eq!(utc_date(NOW + 14 * 3600 - 1), "2026-10-07");
        assert_eq!(utc_date(NOW + 14 * 3600), "2026-10-08");
    }

    #[test]
    fn redaction() {
        assert_eq!(redact("hunter2", true), "[redacted, 7 chars]");
        assert_eq!(redact("hello", false), "hello");
        let long = "é".repeat(MAX_DETAIL_CHARS + 10);
        let short = redact(&long, false);
        assert_eq!(short.chars().count(), MAX_DETAIL_CHARS + 1);
        assert!(short.ends_with('…'));
    }

    #[test]
    fn append_and_read_back_by_day() {
        let dir = test_dir("audit");
        let log = AuditLog::new(&dir);
        let mut yesterday = Entry::new(NOW - DAY, Kind::Stop);
        yesterday.reason = Some("test".into());
        log.append(&yesterday).unwrap();
        let mut today = Entry::new(NOW, Kind::Action);
        today.agent = Some("ion".into());
        today.action = Some(Action::Interact);
        today.ok = Some(true);
        log.append(&today).unwrap();
        log.append(&Entry::new(NOW + 1, Kind::Resume)).unwrap();

        assert!(dir.join("2026-10-06.jsonl").exists());
        assert!(dir.join("2026-10-07.jsonl").exists());
        assert_eq!(log.read_since(0).unwrap().len(), 3);
        let since_now = log.read_since(NOW).unwrap();
        assert_eq!(since_now, vec![today, Entry::new(NOW + 1, Kind::Resume)]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn torn_lines_are_skipped() {
        let dir = test_dir("torn");
        let log = AuditLog::new(&dir);
        log.append(&Entry::new(NOW, Kind::Stop)).unwrap();
        let file = dir.join("2026-10-07.jsonl");
        let mut text = fs::read_to_string(&file).unwrap();
        text.push_str("{\"time\":1,\"ki");
        fs::write(&file, text).unwrap();
        assert_eq!(log.read_since(0).unwrap().len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn entries_skip_empty_fields() {
        let json = serde_json::to_string(&Entry::new(5, Kind::Stop)).unwrap();
        assert_eq!(json, r#"{"time":5,"kind":"stop"}"#);
    }

    #[test]
    fn prune_keeps_the_retention_window() {
        let dir = test_dir("prune");
        let mut log = AuditLog::new(&dir);
        log.set_retention_days(2);
        for days_ago in 0..4 {
            log.append(&Entry::new(NOW - days_ago * DAY, Kind::Stop))
                .unwrap();
        }
        fs::write(dir.join("notes.txt"), "keep me").unwrap();
        assert_eq!(log.prune(NOW).unwrap(), 2);
        assert!(dir.join("2026-10-07.jsonl").exists());
        assert!(dir.join("2026-10-06.jsonl").exists());
        assert!(!dir.join("2026-10-05.jsonl").exists());
        assert!(dir.join("notes.txt").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_missing_directory_is_an_empty_log() {
        let log = AuditLog::new(test_dir("missing"));
        assert!(log.read_since(0).unwrap().is_empty());
        assert_eq!(log.prune(NOW).unwrap(), 0);
    }
}
