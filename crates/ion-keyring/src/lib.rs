//! Secrets for Ion in the operating system's keyring: the Secret Service
//! (GNOME Keyring, KWallet, KeePassXC…) on Linux and the login Keychain on
//! macOS.
//!
//! Anything in Ion that holds a secret (the password manager's encryption key,
//! an agent's API key) stores it here by name:
//!
//! ```no_run
//! let keyring = ion_keyring::global();
//! keyring.set("ai.apiKey", "sk-…")?;
//! assert_eq!(keyring.get("ai.apiKey")?.as_deref(), Some("sk-…"));
//! # Ok::<(), ion_keyring::Error>(())
//! ```
//!
//! When no keyring answers (a desktop without a Secret Service, a headless
//! session), secrets go to a private file instead (`secrets.json`, mode 0600,
//! in Ion's data folder) and [`Keyring::backend`] says so, so the UI can tell
//! people their secrets are only as safe as their home folder.
//! `ION_KEYRING=file` forces the file, for tests and portable setups.

use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Service name secrets are filed under in the OS keyring.
pub const SERVICE: &str = "Ion";

/// Environment variable that picks the store: `file` forces the file store.
pub const BACKEND_ENV: &str = "ION_KEYRING";

/// Where secrets end up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// The Secret Service on Linux.
    SecretService,
    /// The login Keychain on macOS.
    Keychain,
    /// A private file in Ion's data folder: no keyring answered.
    File,
}

impl Backend {
    /// How the store is named to people: "your system keyring", "Keychain".
    pub fn label(self) -> &'static str {
        match self {
            Self::SecretService => "system keyring",
            Self::Keychain => "Keychain",
            Self::File => "a private file",
        }
    }

    /// Whether secrets are protected by the OS rather than file permissions.
    pub fn is_os_keyring(self) -> bool {
        self != Self::File
    }
}

#[derive(Debug)]
pub enum Error {
    /// The keyring refused or failed (locked and the unlock was cancelled,
    /// the service went away).
    Keyring(String),
    /// The file store could not be read or written.
    Io(std::io::Error),
    /// No data folder to keep the file store in.
    NoDataDir,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keyring(err) => write!(f, "keyring: {err}"),
            Self::Io(err) => write!(f, "secrets file: {err}"),
            Self::NoDataDir => f.write_str("no data folder for the secrets file"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// A named-secret store. Use [`global`] in the app; [`Keyring::file`] and
/// [`Keyring::memory`] make private ones for tests.
pub struct Keyring {
    store: Store,
}

enum Store {
    Os(Backend),
    File(FileStore),
    Memory(Mutex<BTreeMap<String, String>>),
}

impl Keyring {
    /// The OS keyring if one answers, else a file in `fallback_dir`.
    ///
    /// Probing reads a secret that doesn't exist, which needs no unlock
    /// prompt; the first real read may ask to unlock the keyring.
    pub fn open(fallback_dir: Option<&Path>) -> Self {
        let file = |dir: Option<&Path>| match dir {
            Some(dir) => Self::file(dir),
            None => Self::memory(),
        };
        if std::env::var(BACKEND_ENV).is_ok_and(|v| v == "file") {
            return file(fallback_dir);
        }
        match os_backend() {
            Some(backend) if probe() => Self {
                store: Store::Os(backend),
            },
            _ => file(fallback_dir),
        }
    }

    /// Secrets in `dir/secrets.json`, readable only by the user.
    pub fn file(dir: &Path) -> Self {
        Self {
            store: Store::File(FileStore {
                path: dir.join("secrets.json"),
                lock: Mutex::new(()),
            }),
        }
    }

    /// Secrets kept in memory only, gone when Ion quits.
    pub fn memory() -> Self {
        Self {
            store: Store::Memory(Mutex::default()),
        }
    }

    pub fn backend(&self) -> Backend {
        match &self.store {
            Store::Os(backend) => *backend,
            Store::File(_) | Store::Memory(_) => Backend::File,
        }
    }

    /// The secret called `name`, or `None` if there is none.
    pub fn get(&self, name: &str) -> Result<Option<String>> {
        match &self.store {
            Store::Os(_) => match entry(name)?.get_password() {
                Ok(secret) => Ok(Some(secret)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(err) => Err(Error::Keyring(err.to_string())),
            },
            Store::File(file) => file.get(name),
            Store::Memory(map) => Ok(map
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(name)
                .cloned()),
        }
    }

    /// Store `secret` as `name`, replacing any earlier one.
    pub fn set(&self, name: &str, secret: &str) -> Result<()> {
        match &self.store {
            Store::Os(_) => entry(name)?
                .set_password(secret)
                .map_err(|err| Error::Keyring(err.to_string())),
            Store::File(file) => file.set(name, Some(secret)),
            Store::Memory(map) => {
                map.lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(name.to_owned(), secret.to_owned());
                Ok(())
            }
        }
    }

    /// Forget the secret called `name`. Not an error if there was none.
    pub fn delete(&self, name: &str) -> Result<()> {
        match &self.store {
            Store::Os(_) => match entry(name)?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(err) => Err(Error::Keyring(err.to_string())),
            },
            Store::File(file) => file.set(name, None),
            Store::Memory(map) => {
                map.lock().unwrap_or_else(|e| e.into_inner()).remove(name);
                Ok(())
            }
        }
    }

    /// The secret called `name`, created with `make` (and stored) if there is
    /// none yet. For keys that must stay the same across runs.
    pub fn get_or_create(&self, name: &str, make: impl FnOnce() -> String) -> Result<String> {
        if let Some(secret) = self.get(name)? {
            return Ok(secret);
        }
        let secret = make();
        self.set(name, &secret)?;
        Ok(secret)
    }
}

/// The keyring shared by the whole app, opened on first use with Ion's data
/// folder (`ION_DATA_DIR` or the platform default) as the fallback.
pub fn global() -> &'static Keyring {
    static GLOBAL: OnceLock<Keyring> = OnceLock::new();
    GLOBAL.get_or_init(|| {
        let keyring = Keyring::open(data_dir().as_deref());
        if !keyring.backend().is_os_keyring() {
            eprintln!("ion: no system keyring answered; keeping secrets in a private file");
        }
        keyring
    })
}

/// Ion's data folder, as `ion_session::paths::data_dir` finds it (repeated
/// here so this crate stays dependency-free for other crates).
fn data_dir() -> Option<PathBuf> {
    let env = |key: &str| std::env::var_os(key).filter(|v| !v.is_empty());
    if let Some(dir) = env("ION_DATA_DIR") {
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

fn os_backend() -> Option<Backend> {
    if cfg!(target_os = "macos") {
        Some(Backend::Keychain)
    } else if cfg!(target_os = "linux") {
        Some(Backend::SecretService)
    } else {
        None
    }
}

fn entry(name: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, name).map_err(|err| Error::Keyring(err.to_string()))
}

/// Whether the OS keyring answers at all.
fn probe() -> bool {
    match keyring::Entry::new(SERVICE, "ion-keyring-probe") {
        Ok(entry) => matches!(entry.get_password(), Ok(_) | Err(keyring::Error::NoEntry)),
        Err(_) => false,
    }
}

struct FileStore {
    path: PathBuf,
    lock: Mutex<()>,
}

impl FileStore {
    fn read(&self) -> Result<BTreeMap<String, String>> {
        match std::fs::read(&self.path) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes).unwrap_or_default()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(err) => Err(err.into()),
        }
    }

    fn get(&self, name: &str) -> Result<Option<String>> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        Ok(self.read()?.remove(name))
    }

    fn set(&self, name: &str, secret: Option<&str>) -> Result<()> {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut map = self.read()?;
        match secret {
            Some(secret) => map.insert(name.to_owned(), secret.to_owned()),
            None => map.remove(name),
        };
        let json = serde_json::to_vec_pretty(&map).map_err(std::io::Error::other)?;
        write_private(&self.path, &json)?;
        Ok(())
    }
}

/// Write `contents` to `path` atomically, readable by the user only.
pub fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.create(parent)?;
    }
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = path.with_extension(format!("{}.{n}.tmp", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let result = (|| {
        let mut file = options.open(&tmp)?;
        file.write_all(contents)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ion-keyring-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn file_store_round_trips_and_deletes() {
        let dir = temp_dir("file");
        let keyring = Keyring::file(&dir);
        assert_eq!(keyring.backend(), Backend::File);
        assert_eq!(keyring.get("a").unwrap(), None);
        keyring.set("a", "one").unwrap();
        keyring.set("b", "two").unwrap();
        keyring.set("a", "three").unwrap();
        // A second store on the same file sees the same secrets.
        let again = Keyring::file(&dir);
        assert_eq!(again.get("a").unwrap().as_deref(), Some("three"));
        again.delete("a").unwrap();
        again.delete("missing").unwrap();
        assert_eq!(keyring.get("a").unwrap(), None);
        assert_eq!(keyring.get("b").unwrap().as_deref(), Some("two"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn file_store_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("private");
        Keyring::file(&dir).set("a", "secret").unwrap();
        let mode = std::fs::metadata(dir.join("secrets.json"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn get_or_create_keeps_the_first_secret() {
        let keyring = Keyring::memory();
        let first = keyring.get_or_create("key", || "made".into()).unwrap();
        let second = keyring.get_or_create("key", || "other".into()).unwrap();
        assert_eq!(first, "made");
        assert_eq!(second, "made");
    }
}
