//! Where Ion keeps the files this crate writes.

use std::path::PathBuf;

/// Environment variable that overrides the data directory (tests, portable setups).
pub const DATA_DIR_ENV: &str = "ION_DATA_DIR";

/// Ion's per-user data directory.
///
/// `$ION_DATA_DIR` if set, else `~/Library/Application Support/Ion` on macOS and
/// `$XDG_DATA_HOME/ion` (default `~/.local/share/ion`) elsewhere. `None` only when
/// no home directory can be found.
pub fn data_dir() -> Option<PathBuf> {
    let env = |key: &str| std::env::var_os(key).filter(|v| !v.is_empty());
    if let Some(dir) = env(DATA_DIR_ENV) {
        return Some(PathBuf::from(dir));
    }
    let home = env("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        return home.map(|h| h.join("Library/Application Support/Ion"));
    }
    env("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home.map(|h| h.join(".local/share")))
        .map(|d| d.join("ion"))
}

/// Write `contents` to `path` atomically: readers see the old file or the new
/// one, never a half-written file if Ion crashes mid-save.
pub(crate) fn write_atomic(path: &std::path::Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
pub(crate) fn test_dir(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ion-session-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_creates_parents_and_replaces() {
        let dir = test_dir("atomic");
        let path = dir.join("a/b/file.json");
        write_atomic(&path, b"one").unwrap();
        write_atomic(&path, b"two").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"two");
        assert!(!dir.join("a/b/file.json.tmp").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
