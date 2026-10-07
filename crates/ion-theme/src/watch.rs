//! Live reload: watch a palette file and report when it changes.

use std::ffi::OsString;
use std::path::Path;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Calls back whenever the watched file is created, written, replaced or
/// removed. Watching stops when this is dropped.
///
/// The parent directory is watched rather than the file, because tools like
/// matugen write a new file and rename it over the old one, which would end a
/// watch on the file itself. The callback runs on a background thread.
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    pub fn new(path: &Path, on_change: impl Fn() + Send + 'static) -> notify::Result<Self> {
        let name: OsString = path
            .file_name()
            .ok_or_else(|| notify::Error::generic("palette path has no file name"))?
            .to_owned();
        let dir = match path.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => dir.to_owned(),
            _ => std::env::current_dir()?,
        };
        // The source may be configured before the theme tool first runs.
        std::fs::create_dir_all(&dir)?;

        let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            let Ok(event) = event else { return };
            if matches!(event.kind, EventKind::Access(_)) {
                return;
            }
            if event
                .paths
                .iter()
                .any(|p| p.file_name() == Some(name.as_os_str()))
            {
                on_change();
            }
        })?;
        watcher.watch(&dir, RecursiveMode::NonRecursive)?;
        Ok(Self { _watcher: watcher })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_dir;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn reports_writes_and_atomic_replaces() {
        let dir = test_dir("watch");
        let file = dir.join("palette.toml");
        let (tx, rx) = mpsc::channel();
        let _watcher = FileWatcher::new(&file, move || {
            let _ = tx.send(());
        })
        .unwrap();
        let wait = || rx.recv_timeout(Duration::from_secs(10)).is_ok();
        let drain = || while rx.try_recv().is_ok() {};

        // Unrelated files in the same directory are ignored.
        std::fs::write(dir.join("other.toml"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(300));
        drain();

        std::fs::write(&file, "one").unwrap();
        assert!(wait(), "no event for a direct write");
        std::thread::sleep(Duration::from_millis(300));
        drain();

        let tmp = dir.join(".palette.toml.tmp");
        std::fs::write(&tmp, "two").unwrap();
        std::fs::rename(&tmp, &file).unwrap();
        assert!(wait(), "no event for an atomic replace");

        std::fs::remove_dir_all(dir).unwrap();
    }
}
