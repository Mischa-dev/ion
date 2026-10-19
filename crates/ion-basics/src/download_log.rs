//! The list behind the Downloads page (`ion://downloads`): every download
//! that finished, failed or was cancelled, kept across restarts in
//! `<data dir>/downloads.json`. Downloads still running live in the
//! downloads panel; they are recorded here once they end.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Downloads kept before the oldest are dropped.
pub const MAX_RECORDS: usize = 1000;

/// One past download.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// Unique within the log; the page refers to rows by it.
    pub id: u64,
    /// Where it was downloaded from.
    pub url: String,
    /// The folder it was saved in.
    pub directory: String,
    pub file_name: String,
    #[serde(default)]
    pub mime_type: String,
    /// Size in bytes; 0 when unknown.
    #[serde(default)]
    pub bytes: u64,
    pub outcome: Outcome,
    /// Unix time (seconds) it ended.
    pub finished: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Completed,
    Cancelled,
    Failed,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Completed, Self::Cancelled, Self::Failed]
            .into_iter()
            .find(|o| o.as_str() == name)
    }
}

impl Record {
    pub fn path(&self) -> PathBuf {
        Path::new(&self.directory).join(&self.file_name)
    }
}

#[derive(Default, Serialize, Deserialize)]
struct LogFile {
    #[serde(default)]
    downloads: Vec<Record>,
}

/// Past downloads, oldest first.
#[derive(Debug, Default)]
pub struct DownloadLog {
    records: Vec<Record>,
}

impl DownloadLog {
    /// Read the log at `path`; a missing file is an empty log.
    pub fn load(path: &Path) -> io::Result<Self> {
        let file: LogFile = match std::fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => LogFile::default(),
            Err(e) => return Err(e),
        };
        Ok(Self {
            records: file.downloads,
        })
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec(&LogFile {
            downloads: self.records.clone(),
        })
        .map_err(io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, path)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Add a download that ended. Returns its id.
    pub fn record(&mut self, mut record: Record) -> u64 {
        record.id = self
            .records
            .iter()
            .map(|r| r.id)
            .max()
            .map_or(1, |id| id + 1);
        let id = record.id;
        self.records.push(record);
        if self.records.len() > MAX_RECORDS {
            let extra = self.records.len() - MAX_RECORDS;
            self.records.drain(..extra);
        }
        id
    }

    /// Forget one download (the file stays). True if it was there.
    pub fn remove(&mut self, id: u64) -> bool {
        let before = self.records.len();
        self.records.retain(|r| r.id != id);
        self.records.len() != before
    }

    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Downloads whose file name or address contains every word of
    /// `query`, newest first.
    pub fn search(&self, query: &str) -> Vec<&Record> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let mut hits: Vec<&Record> = self
            .records
            .iter()
            .filter(|r| {
                let haystack = format!("{} {}", r.file_name, r.url).to_lowercase();
                words.iter().all(|w| haystack.contains(w.as_str()))
            })
            .collect();
        hits.sort_by(|a, b| b.finished.cmp(&a.finished).then(b.id.cmp(&a.id)));
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &str, finished: u64) -> Record {
        Record {
            id: 0,
            url: format!("https://files.example/{name}"),
            directory: "/tmp/downloads".into(),
            file_name: name.into(),
            mime_type: String::new(),
            bytes: 10,
            outcome: Outcome::Completed,
            finished,
        }
    }

    #[test]
    fn records_get_ids_and_search_is_newest_first() {
        let mut log = DownloadLog::default();
        let a = log.record(record("report.pdf", 1));
        let b = log.record(record("photo.png", 2));
        assert_ne!(a, b);
        let names: Vec<&str> = log
            .search("")
            .iter()
            .map(|r| r.file_name.as_str())
            .collect();
        assert_eq!(names, ["photo.png", "report.pdf"]);
        assert_eq!(log.search("REPORT").len(), 1);
        assert_eq!(log.search("files.example png").len(), 1);
        assert!(log.remove(a));
        assert!(!log.remove(a));
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn oldest_go_past_the_cap() {
        let mut log = DownloadLog::default();
        for i in 0..MAX_RECORDS + 5 {
            log.record(record(&format!("{i}.bin"), i as u64));
        }
        assert_eq!(log.len(), MAX_RECORDS);
        assert_eq!(log.search("").last().unwrap().file_name, "5.bin");
    }

    #[test]
    fn saves_and_loads() {
        let dir = std::env::temp_dir().join(format!("ion-download-log-{}", std::process::id()));
        let path = dir.join("downloads.json");
        let mut log = DownloadLog::default();
        log.record(record("a.zip", 3));
        log.save(&path).unwrap();
        let loaded = DownloadLog::load(&path).unwrap();
        assert_eq!(loaded.search("")[0].file_name, "a.zip");
        assert_eq!(
            loaded.search("")[0].path(),
            Path::new("/tmp/downloads/a.zip")
        );
        assert!(
            DownloadLog::load(&dir.join("missing.json"))
                .unwrap()
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn outcome_names_round_trip() {
        for o in [Outcome::Completed, Outcome::Cancelled, Outcome::Failed] {
            assert_eq!(Outcome::from_name(o.as_str()), Some(o));
        }
    }
}
