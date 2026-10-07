//! On-disk cache: downloaded filter lists and the compiled engine.
//!
//! ```text
//! <dir>/lists/<id>.txt   one file per filter list, as downloaded
//! <dir>/engine.dat       the compiled engine, tagged with the lists it was built from
//! ```

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::{Blocker, FilterList};

/// Bump when the engine file layout or the adblock-rust major version changes.
const ENGINE_FORMAT: &str = "ion-adblock-engine/1 adblock/0.13";

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The platform cache directory, e.g. `~/.cache/ion/adblock` on Linux.
    pub fn default_dir() -> Option<PathBuf> {
        Some(dirs::cache_dir()?.join("ion").join("adblock"))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn list_path(&self, id: &str) -> PathBuf {
        self.dir.join("lists").join(format!("{id}.txt"))
    }

    fn engine_path(&self) -> PathBuf {
        self.dir.join("engine.dat")
    }

    pub fn read_list(&self, id: &str) -> Option<String> {
        std::fs::read_to_string(self.list_path(id)).ok()
    }

    pub fn write_list(&self, id: &str, text: &str) -> io::Result<()> {
        write_atomic(&self.list_path(id), text.as_bytes())
    }

    /// When the list was last downloaded, if it is cached.
    pub fn list_modified(&self, id: &str) -> Option<SystemTime> {
        std::fs::metadata(self.list_path(id)).ok()?.modified().ok()
    }

    /// How long ago the list was downloaded; `None` if it isn't cached.
    pub fn list_age(&self, id: &str, now: SystemTime) -> Option<Duration> {
        let modified = self.list_modified(id)?;
        Some(now.duration_since(modified).unwrap_or_default())
    }

    /// The most recent download among `lists`.
    pub fn last_updated(&self, lists: &[FilterList]) -> Option<SystemTime> {
        lists.iter().filter_map(|l| self.list_modified(&l.id)).max()
    }

    /// Identifies a set of cached lists, so a compiled engine is only reused
    /// for exactly the lists (and download times) it was built from.
    fn engine_key(&self, lists: &[FilterList]) -> String {
        let mut key = String::from(ENGINE_FORMAT);
        for list in lists {
            let meta = std::fs::metadata(self.list_path(&list.id)).ok();
            let len = meta.as_ref().map_or(0, |m| m.len());
            let stamp = meta
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_nanos());
            key.push_str(&format!(" {}@{stamp}:{len}", list.id));
        }
        key
    }

    /// Save a compiled engine built from the currently cached `lists`.
    pub fn write_engine(&self, lists: &[FilterList], blocker: &Blocker) -> io::Result<()> {
        let mut bytes = self.engine_key(lists).into_bytes();
        bytes.push(b'\n');
        bytes.extend_from_slice(&blocker.serialize());
        write_atomic(&self.engine_path(), &bytes)
    }

    /// The cached engine, if it exists and matches the cached `lists`.
    pub fn read_engine(&self, lists: &[FilterList]) -> Option<Blocker> {
        let bytes = std::fs::read(self.engine_path()).ok()?;
        let split = bytes.iter().position(|&b| b == b'\n')?;
        let (key, rest) = bytes.split_at(split);
        if key != self.engine_key(lists).as_bytes() {
            return None;
        }
        Blocker::deserialize(&rest[1..]).ok()
    }
}

/// Write via a temporary file and rename, so a crash never leaves a half
/// written list or engine behind.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Unique per writer, so two refreshes racing never share a temp file.
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = path.with_extension(format!("{}.{n}.tmp", std::process::id()));
    let mut file = std::fs::File::create(&tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{RequestInfo, ResourceType};

    /// A fresh, empty directory under the system temp dir.
    pub(crate) fn temp_dir(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ion-adblock-test-{}-{name}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn lists() -> Vec<FilterList> {
        vec![FilterList::new("one", "One", "https://lists.test/one.txt")]
    }

    #[test]
    fn lists_round_trip() {
        let store = Store::new(temp_dir("lists"));
        assert_eq!(store.read_list("one"), None);
        assert_eq!(store.list_age("one", SystemTime::now()), None);
        store.write_list("one", "||ads.test^\n").unwrap();
        assert_eq!(store.read_list("one").as_deref(), Some("||ads.test^\n"));
        assert!(store.list_age("one", SystemTime::now()).unwrap() < Duration::from_secs(60));
        assert!(store.last_updated(&lists()).is_some());
    }

    #[test]
    fn engine_is_reused_only_for_the_same_lists() {
        let store = Store::new(temp_dir("engine"));
        let lists = lists();
        store.write_list("one", "||ads.test^\n").unwrap();
        let blocker = Blocker::from_lists(["||ads.test^\n"]);
        store.write_engine(&lists, &blocker).unwrap();

        let loaded = store.read_engine(&lists).expect("cached engine");
        let req = RequestInfo::new(
            "https://ads.test/a.js",
            "https://site.test/",
            ResourceType::Script,
        );
        assert!(loaded.should_block(&req));

        // A different set of lists must not reuse it.
        let more = vec![
            lists[0].clone(),
            FilterList::new("two", "Two", "https://lists.test/two.txt"),
        ];
        assert!(store.read_engine(&more).is_none());

        // Neither may a re-downloaded list.
        store.write_list("one", "||other.test^\n").unwrap();
        assert!(store.read_engine(&lists).is_none());
    }

    #[test]
    fn corrupt_engine_is_ignored() {
        let store = Store::new(temp_dir("corrupt"));
        let lists = lists();
        let key = store.engine_key(&lists);
        write_atomic(
            &store.engine_path(),
            format!("{key}\nnot an engine").as_bytes(),
        )
        .unwrap();
        assert!(store.read_engine(&lists).is_none());
    }
}
