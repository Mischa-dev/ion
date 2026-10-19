//! Ion's password manager, without Qt.
//!
//! Saved logins live in `passwords.json` in Ion's data folder. Passwords and
//! notes in it are encrypted with AES-256-GCM under a key kept in the OS
//! keyring ([`ion_keyring`]), the way Chrome keeps its "Safe Storage" key, so
//! the file alone gives nothing away and the keyring holds one item rather
//! than one per site.
//!
//! The vault decides what the browser offers on a page ([`Vault::matches`]),
//! whether a submitted form is worth a save or update prompt
//! ([`Vault::propose`]), and reads and writes the CSV files other browsers
//! export ([`Vault::import_csv`], [`Vault::export_csv`]).

pub mod csv;
pub mod generate;
pub mod origin;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};

pub use generate::{DEFAULT_LENGTH, generate, is_weak};
pub use origin::{Fit, display_host, origin_of, site_of};

/// The script that finds login forms in pages, reports them to Ion and fills
/// them. Install it in an isolated world of the top frame, after
/// `qwebchannel.js`, with a web channel object registered as `ionForms`
/// whose methods take the page URL first: `formFound(url)`,
/// `fieldFocused(url, kind, x, y, width, height)`, `fieldBlurred(url)`,
/// `menuKey(url, key)` and `submitted(url, username, password)`. Ion calls
/// `window.__ionForms.fill(username, password, onlyEmpty)`,
/// `.fillNew(password)` and `.menu(open)` back in the same world.
pub const PAGE_SCRIPT: &str = include_str!("forms.js");

/// The keyring item holding the vault's key.
pub const KEY_NAME: &str = "passwords.key";

/// One saved login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Login {
    pub id: u64,
    /// `scheme://host[:port]` of the page it was saved on (see [`origin_of`]).
    pub origin: String,
    pub username: String,
    pub password: String,
    pub note: String,
    /// Milliseconds since the Unix epoch.
    pub created: i64,
    /// When the password last changed.
    pub changed: i64,
    /// When it was last filled in, 0 for never.
    pub used: i64,
    pub times_used: u32,
}

/// A saved login offered on a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match<'a> {
    pub login: &'a Login,
    pub fit: Fit,
}

/// What to ask after someone submits a form with a password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proposal {
    /// Nothing: already saved as is, or the site is on the never-save list.
    Nothing,
    /// Offer to save a new login.
    Save,
    /// Offer to replace the password of the login with this id.
    Update(u64),
}

/// What an import did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub added: usize,
    pub updated: usize,
    /// Already saved exactly, or not a web login (an Android app entry).
    pub skipped: usize,
}

/// Logins worth a second look.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Checkup {
    /// Short, simple or very common passwords.
    pub weak: Vec<u64>,
    /// Passwords used for more than one site.
    pub reused: Vec<u64>,
}

#[derive(Debug)]
pub enum Error {
    /// The keyring didn't give out the key.
    Keyring(ion_keyring::Error),
    /// The file couldn't be read or written.
    Io(std::io::Error),
    /// The file is damaged, or was encrypted with another key (the keyring
    /// item was deleted or the file copied from another machine).
    Undecryptable,
    Csv(csv::CsvError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keyring(err) => write!(f, "{err}"),
            Self::Io(err) => write!(f, "{err}"),
            Self::Undecryptable => f.write_str(
                "saved passwords can't be decrypted: the key in the keyring doesn't match",
            ),
            Self::Csv(err) => write!(f, "{err}"),
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

/// The saved logins, decrypted in memory, written back on every change.
pub struct Vault {
    path: Option<PathBuf>,
    cipher: Aes256Gcm,
    logins: Vec<Login>,
    /// Origins where Ion never offers to save.
    never: BTreeSet<String>,
    next_id: u64,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct File {
    version: u32,
    logins: Vec<StoredLogin>,
    never: BTreeSet<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredLogin {
    id: u64,
    origin: String,
    username: String,
    /// base64 of nonce ‖ ciphertext.
    password: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    note: String,
    created: i64,
    changed: i64,
    #[serde(default)]
    used: i64,
    #[serde(default)]
    times_used: u32,
}

impl Vault {
    /// The vault in `dir/passwords.json`, its key from `keyring` (made on
    /// first use).
    pub fn open(dir: &Path, keyring: &ion_keyring::Keyring) -> Result<Self> {
        let encoded = keyring
            .get_or_create(KEY_NAME, || {
                let mut key = [0u8; 32];
                getrandom::fill(&mut key).expect("the OS random source failed");
                BASE64.encode(key)
            })
            .map_err(Error::Keyring)?;
        let key: [u8; 32] = BASE64
            .decode(encoded.trim())
            .ok()
            .and_then(|k| k.try_into().ok())
            .ok_or(Error::Undecryptable)?;
        Self::open_with_key(Some(dir.join("passwords.json")), key)
    }

    /// An empty vault that is never written to disk.
    pub fn in_memory() -> Self {
        let mut key = [0u8; 32];
        getrandom::fill(&mut key).expect("the OS random source failed");
        Self::open_with_key(None, key).expect("an in-memory vault reads no file")
    }

    fn open_with_key(path: Option<PathBuf>, key: [u8; 32]) -> Result<Self> {
        let cipher = Aes256Gcm::new(&key.into());
        let file: File = match path.as_ref().map(std::fs::read) {
            Some(Ok(bytes)) => serde_json::from_slice(&bytes).map_err(|_| Error::Undecryptable)?,
            Some(Err(err)) if err.kind() != std::io::ErrorKind::NotFound => return Err(err.into()),
            _ => File::default(),
        };
        let mut vault = Self {
            path,
            cipher,
            logins: Vec::with_capacity(file.logins.len()),
            never: file.never,
            next_id: 1,
        };
        for stored in file.logins {
            let password = vault.decrypt(&stored.password)?;
            let note = if stored.note.is_empty() {
                String::new()
            } else {
                vault.decrypt(&stored.note)?
            };
            vault.next_id = vault.next_id.max(stored.id + 1);
            vault.logins.push(Login {
                id: stored.id,
                origin: stored.origin,
                username: stored.username,
                password,
                note,
                created: stored.created,
                changed: stored.changed,
                used: stored.used,
                times_used: stored.times_used,
            });
        }
        Ok(vault)
    }

    fn encrypt(&self, plain: &str) -> String {
        let mut nonce = [0u8; 12];
        getrandom::fill(&mut nonce).expect("the OS random source failed");
        let sealed = self
            .cipher
            .encrypt(Nonce::from_slice(&nonce), plain.as_bytes())
            .expect("AES-GCM encryption of a short string cannot fail");
        let mut out = nonce.to_vec();
        out.extend(sealed);
        BASE64.encode(out)
    }

    fn decrypt(&self, sealed: &str) -> Result<String> {
        let bytes = BASE64.decode(sealed).map_err(|_| Error::Undecryptable)?;
        if bytes.len() < 12 {
            return Err(Error::Undecryptable);
        }
        let (nonce, body) = bytes.split_at(12);
        let plain = self
            .cipher
            .decrypt(Nonce::from_slice(nonce), body)
            .map_err(|_| Error::Undecryptable)?;
        String::from_utf8(plain).map_err(|_| Error::Undecryptable)
    }

    fn persist(&self) -> Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let file = File {
            version: 1,
            logins: self
                .logins
                .iter()
                .map(|login| StoredLogin {
                    id: login.id,
                    origin: login.origin.clone(),
                    username: login.username.clone(),
                    password: self.encrypt(&login.password),
                    note: if login.note.is_empty() {
                        String::new()
                    } else {
                        self.encrypt(&login.note)
                    },
                    created: login.created,
                    changed: login.changed,
                    used: login.used,
                    times_used: login.times_used,
                })
                .collect(),
            never: self.never.clone(),
        };
        let json = serde_json::to_vec_pretty(&file).map_err(std::io::Error::other)?;
        ion_keyring::write_private(path, &json)?;
        Ok(())
    }

    /// Every saved login, in the order they were saved.
    pub fn logins(&self) -> &[Login] {
        &self.logins
    }

    pub fn get(&self, id: u64) -> Option<&Login> {
        self.logins.iter().find(|l| l.id == id)
    }

    /// Logins to offer on the page at `url`: its own first (most recently
    /// used first), then other subdomains of the same site.
    pub fn matches(&self, url: &str) -> Vec<Match<'_>> {
        let Some(page) = origin_of(url) else {
            return Vec::new();
        };
        let mut found: Vec<Match> = self
            .logins
            .iter()
            .filter_map(|login| origin::fit(&login.origin, &page).map(|fit| Match { login, fit }))
            .collect();
        found.sort_by(|a, b| {
            b.fit
                .cmp(&a.fit)
                .then(b.login.used.cmp(&a.login.used))
                .then(a.login.username.cmp(&b.login.username))
        });
        found
    }

    /// The one login to fill in by itself when the page loads: only when the
    /// page's own origin has exactly one.
    pub fn autofill_candidate(&self, url: &str) -> Option<&Login> {
        let matches = self.matches(url);
        let mut exact = matches.iter().filter(|m| m.fit == Fit::Exact);
        match (exact.next(), exact.next()) {
            (Some(only), None) => Some(only.login),
            _ => None,
        }
    }

    /// What to ask after a form on `url` was sent with `username` and
    /// `password`.
    pub fn propose(&self, url: &str, username: &str, password: &str) -> Proposal {
        let Some(page) = origin_of(url) else {
            return Proposal::Nothing;
        };
        if password.is_empty() || self.never.contains(&page) {
            return Proposal::Nothing;
        }
        let exact: Vec<&Login> = self
            .matches(url)
            .into_iter()
            .filter(|m| m.fit == Fit::Exact)
            .map(|m| m.login)
            .collect();
        if exact
            .iter()
            .any(|l| l.password == password && (username.is_empty() || l.username == username))
        {
            return Proposal::Nothing;
        }
        if let Some(same_user) = exact.iter().find(|l| l.username == username) {
            return Proposal::Update(same_user.id);
        }
        // A change-password form often has no username field: with one
        // login for the site, that's the one being changed.
        if username.is_empty() && exact.len() == 1 {
            return Proposal::Update(exact[0].id);
        }
        Proposal::Save
    }

    /// Save a login for the page at `url`, or change the password of the one
    /// already saved there for `username`. Returns its id.
    pub fn save(&mut self, url: &str, username: &str, password: &str, now: i64) -> Result<u64> {
        let origin = origin_of(url).unwrap_or_else(|| url.trim().to_owned());
        let id = self
            .upsert(origin, username, password, String::new(), now)
            .0;
        self.never.remove(&origin_of(url).unwrap_or_default());
        self.persist()?;
        Ok(id)
    }

    /// Returns the id and whether anything changed.
    fn upsert(
        &mut self,
        origin: String,
        username: &str,
        password: &str,
        note: String,
        now: i64,
    ) -> (u64, Change) {
        if let Some(login) = self
            .logins
            .iter_mut()
            .find(|l| l.origin == origin && l.username == username)
        {
            if login.password == password {
                return (login.id, Change::None);
            }
            login.password = password.to_owned();
            login.changed = now;
            if !note.is_empty() {
                login.note = note;
            }
            return (login.id, Change::Updated);
        }
        let id = self.next_id;
        self.next_id += 1;
        self.logins.push(Login {
            id,
            origin,
            username: username.to_owned(),
            password: password.to_owned(),
            note,
            created: now,
            changed: now,
            used: 0,
            times_used: 0,
        });
        (id, Change::Added)
    }

    /// Edit a saved login. False if there is none with `id`.
    pub fn edit(
        &mut self,
        id: u64,
        username: &str,
        password: &str,
        note: &str,
        now: i64,
    ) -> Result<bool> {
        let Some(login) = self.logins.iter_mut().find(|l| l.id == id) else {
            return Ok(false);
        };
        if login.password != password {
            login.changed = now;
        }
        login.username = username.to_owned();
        login.password = password.to_owned();
        login.note = note.to_owned();
        self.persist()?;
        Ok(true)
    }

    /// Replace only the password of login `id` (an update prompt).
    pub fn set_password(&mut self, id: u64, password: &str, now: i64) -> Result<bool> {
        let Some(login) = self.get(id).cloned() else {
            return Ok(false);
        };
        self.edit(id, &login.username, password, &login.note, now)
    }

    pub fn remove(&mut self, id: u64) -> Result<bool> {
        let before = self.logins.len();
        self.logins.retain(|l| l.id != id);
        let removed = self.logins.len() != before;
        if removed {
            self.persist()?;
        }
        Ok(removed)
    }

    /// Note that login `id` was filled in.
    pub fn mark_used(&mut self, id: u64, now: i64) -> Result<()> {
        if let Some(login) = self.logins.iter_mut().find(|l| l.id == id) {
            login.used = now;
            login.times_used = login.times_used.saturating_add(1);
            self.persist()?;
        }
        Ok(())
    }

    /// Never offer to save passwords on the site at `url`.
    pub fn never_save(&mut self, url: &str) -> Result<()> {
        if let Some(origin) = origin_of(url)
            && self.never.insert(origin)
        {
            self.persist()?;
        }
        Ok(())
    }

    /// Origins on the never-save list.
    pub fn never_saved(&self) -> impl Iterator<Item = &str> {
        self.never.iter().map(String::as_str)
    }

    /// Offer to save on `origin` again.
    pub fn allow_save(&mut self, origin: &str) -> Result<()> {
        if self.never.remove(origin) {
            self.persist()?;
        }
        Ok(())
    }

    /// Add the logins in a CSV export from another browser or manager. A
    /// login already saved for the same site and username takes the
    /// imported password.
    pub fn import_csv(&mut self, text: &str, now: i64) -> Result<ImportReport> {
        let rows = csv::parse(text).map_err(Error::Csv)?;
        let mut report = ImportReport::default();
        for row in rows {
            let Some(origin) = origin_of(&row.url) else {
                report.skipped += 1;
                continue;
            };
            match self
                .upsert(origin, &row.username, &row.password, row.note, now)
                .1
            {
                Change::Added => report.added += 1,
                Change::Updated => report.updated += 1,
                Change::None => report.skipped += 1,
            }
        }
        if report.added + report.updated > 0 {
            self.persist()?;
        }
        Ok(report)
    }

    /// Every login as a CSV in Chrome's format. It holds every password in
    /// plain text.
    pub fn export_csv(&self) -> String {
        let rows: Vec<(String, csv::Row)> = self
            .logins
            .iter()
            .map(|login| {
                (
                    display_host(&login.origin),
                    csv::Row {
                        url: login.origin.clone(),
                        username: login.username.clone(),
                        password: login.password.clone(),
                        note: login.note.clone(),
                    },
                )
            })
            .collect();
        csv::write(&rows)
    }

    /// Weak passwords, and passwords shared between sites.
    pub fn checkup(&self) -> Checkup {
        let mut sites_by_password: HashMap<&str, BTreeSet<String>> = HashMap::new();
        for login in &self.logins {
            sites_by_password
                .entry(&login.password)
                .or_default()
                .insert(site_of(&login.origin));
        }
        Checkup {
            weak: self
                .logins
                .iter()
                .filter(|l| is_weak(&l.password))
                .map(|l| l.id)
                .collect(),
            reused: self
                .logins
                .iter()
                .filter(|l| sites_by_password[l.password.as_str()].len() > 1)
                .map(|l| l.id)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Change {
    None,
    Added,
    Updated,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ion-passwords-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn saves_survive_a_restart_and_are_encrypted_on_disk() {
        let dir = temp_dir("restart");
        let keyring = ion_keyring::Keyring::memory();
        let mut vault = Vault::open(&dir, &keyring).unwrap();
        let id = vault
            .save("https://a.com/login", "alice", "hunter2-secret", 5)
            .unwrap();
        vault.never_save("https://b.com/x").unwrap();
        let text = std::fs::read_to_string(dir.join("passwords.json")).unwrap();
        assert!(!text.contains("hunter2"));
        assert!(text.contains("alice"));

        let vault = Vault::open(&dir, &keyring).unwrap();
        let login = vault.get(id).unwrap();
        assert_eq!(login.password, "hunter2-secret");
        assert_eq!(login.origin, "https://a.com");
        assert_eq!(vault.never_saved().collect::<Vec<_>>(), ["https://b.com"]);

        // Another key can't read it, and says so instead of losing data.
        let other = ion_keyring::Keyring::memory();
        assert!(matches!(
            Vault::open(&dir, &other),
            Err(Error::Undecryptable)
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn proposals_follow_what_is_saved() {
        let mut vault = Vault::in_memory();
        let url = "https://a.com/login";
        assert_eq!(vault.propose(url, "alice", "one"), Proposal::Save);
        assert_eq!(vault.propose(url, "alice", ""), Proposal::Nothing);
        assert_eq!(
            vault.propose("file:///x", "alice", "one"),
            Proposal::Nothing
        );
        let id = vault.save(url, "alice", "one", 1).unwrap();
        assert_eq!(vault.propose(url, "alice", "one"), Proposal::Nothing);
        assert_eq!(vault.propose(url, "", "one"), Proposal::Nothing);
        assert_eq!(vault.propose(url, "alice", "two"), Proposal::Update(id));
        // A change-password form without a username field.
        assert_eq!(vault.propose(url, "", "two"), Proposal::Update(id));
        assert_eq!(vault.propose(url, "bob", "one"), Proposal::Save);
        vault.never_save("https://a.com/").unwrap();
        assert_eq!(vault.propose(url, "bob", "x"), Proposal::Nothing);
        // Saving anyway (from the manager) takes it off the list.
        vault.save(url, "bob", "x", 2).unwrap();
        assert_eq!(vault.never_saved().count(), 0);
    }

    #[test]
    fn matches_put_the_page_first_and_autofill_only_one() {
        let mut vault = Vault::in_memory();
        vault.save("https://login.a.com", "sso", "1", 1).unwrap();
        let first = vault.save("https://a.com", "alice", "2", 1).unwrap();
        assert_eq!(
            vault.autofill_candidate("https://a.com/x").unwrap().id,
            first
        );
        let names: Vec<_> = vault
            .matches("https://a.com/")
            .iter()
            .map(|m| (m.login.username.as_str(), m.fit))
            .collect();
        assert_eq!(names, [("alice", Fit::Exact), ("sso", Fit::SameSite)]);
        let second = vault.save("https://a.com", "bob", "3", 1).unwrap();
        assert!(vault.autofill_candidate("https://a.com/x").is_none());
        vault.mark_used(second, 9).unwrap();
        assert_eq!(vault.matches("https://a.com")[0].login.username, "bob");
        assert!(vault.autofill_candidate("https://other.com").is_none());
    }

    #[test]
    fn edits_and_removals() {
        let mut vault = Vault::in_memory();
        let id = vault.save("https://a.com", "alice", "one", 1).unwrap();
        assert!(vault.edit(id, "al", "two", "note", 7).unwrap());
        let login = vault.get(id).unwrap();
        assert_eq!((login.username.as_str(), login.changed), ("al", 7));
        assert!(vault.set_password(id, "three", 8).unwrap());
        assert_eq!(vault.get(id).unwrap().note, "note");
        assert!(vault.remove(id).unwrap());
        assert!(!vault.remove(id).unwrap());
        assert!(!vault.edit(id, "", "", "", 0).unwrap());
    }

    #[test]
    fn imports_merge_and_exports_round_trip() {
        let mut vault = Vault::in_memory();
        vault.save("https://a.com", "alice", "old", 1).unwrap();
        let csv = "name,url,username,password,note\n\
                   a,https://a.com/login,alice,new,\n\
                   b,https://b.com/,bob,pw,hello\n\
                   b,https://b.com/,bob,pw,hello\n\
                   app,android://hash@com.app/,me,x,\n";
        let report = vault.import_csv(csv, 2).unwrap();
        assert_eq!(
            report,
            ImportReport {
                added: 1,
                updated: 1,
                skipped: 2
            }
        );
        assert_eq!(vault.matches("https://a.com")[0].login.password, "new");

        let mut copy = Vault::in_memory();
        copy.import_csv(&vault.export_csv(), 3).unwrap();
        let pairs = |v: &Vault| {
            v.logins()
                .iter()
                .map(|l| {
                    (
                        l.origin.clone(),
                        l.username.clone(),
                        l.password.clone(),
                        l.note.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(pairs(&copy), pairs(&vault));
        assert!(matches!(vault.import_csv("x,y\n", 0), Err(Error::Csv(_))));
    }

    #[test]
    fn checkup_finds_weak_and_reused() {
        let mut vault = Vault::in_memory();
        let weak = vault.save("https://a.com", "a", "password", 1).unwrap();
        let reused1 = vault
            .save("https://b.com", "b", "Shared-Secret-42!", 1)
            .unwrap();
        let reused2 = vault
            .save("https://c.com", "c", "Shared-Secret-42!", 1)
            .unwrap();
        // The same password on two subdomains of one site isn't reuse.
        vault
            .save("https://x.d.com", "d", "Only-Here-123!", 1)
            .unwrap();
        vault
            .save("https://y.d.com", "d", "Only-Here-123!", 1)
            .unwrap();
        let checkup = vault.checkup();
        assert_eq!(checkup.weak, [weak]);
        assert_eq!(checkup.reused, [reused1, reused2]);
    }
}
