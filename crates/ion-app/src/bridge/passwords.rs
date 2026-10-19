//! `Passwords` QML singleton: Ion's password manager (`ion_passwords`), plus
//! the shared keyring (`ion_keyring`) for other secrets.
//!
//! The vault opens on first use, not at startup, because opening it may ask
//! the system to unlock its keyring; the first login page is a sensible
//! moment for that prompt, a fresh window is not. Passwords never travel in
//! bulk: lists carry usernames and sites, and `password(id)` hands out one at
//! a time for filling, revealing or copying.
//!
//! `revision` changes whenever the saved logins do.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/passwords.h");

        #[cxx_name = "copySecret"]
        fn copy_secret(text: &QString);

        #[cxx_name = "clearSecret"]
        fn clear_secret(text: &QString);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(i32, revision, READ, NOTIFY)]
        #[namespace = "ion"]
        type Passwords = super::PasswordsRust;

        /// Whether Ion offers to save passwords (`passwords.enable`).
        #[qinvokable]
        fn enabled(self: &Passwords) -> bool;

        /// "" when the saved passwords can be used, else why not (the
        /// keyring refused, the file can't be decrypted). Opens the vault.
        #[qinvokable]
        fn problem(self: Pin<&mut Passwords>) -> QString;

        /// Where secrets are kept, for people: "system keyring", "Keychain"
        /// or "a private file".
        #[qinvokable]
        #[cxx_name = "storeName"]
        fn store_name(self: &Passwords) -> QString;

        /// Whether that is the OS keyring rather than a file.
        #[qinvokable]
        #[cxx_name = "storeIsKeyring"]
        fn store_is_keyring(self: &Passwords) -> bool;

        /// Logins to offer on the page at `url`, as a JSON array of
        /// `{id, username, host, exact}`, the page's own first.
        #[qinvokable]
        fn matches(self: Pin<&mut Passwords>, url: &QString) -> QString;

        /// The login to fill in by itself as the page at `url` loads, or -1:
        /// only with `passwords.autofill`, on https (or localhost), and when
        /// the page's own origin has exactly one.
        #[qinvokable]
        #[cxx_name = "autofillFor"]
        fn autofill_for(self: Pin<&mut Passwords>, url: &QString) -> i32;

        /// `{id, username, password}` of login `id` as JSON, or "" if gone.
        #[qinvokable]
        fn credentials(self: Pin<&mut Passwords>, id: i32) -> QString;

        /// What to offer after a form on `url` was sent: JSON
        /// `{kind: "save"|"update"|"", id, username}`.
        #[qinvokable]
        fn propose(
            self: Pin<&mut Passwords>,
            url: &QString,
            username: &QString,
            password: &QString,
        ) -> QString;

        /// Save a login for the page at `url` (or change the password saved
        /// there for `username`). Returns its id, or -1.
        #[qinvokable]
        fn save(
            self: Pin<&mut Passwords>,
            url: &QString,
            username: &QString,
            password: &QString,
        ) -> i32;

        /// Replace the password of login `id`.
        #[qinvokable]
        #[cxx_name = "updatePassword"]
        fn update_password(self: Pin<&mut Passwords>, id: i32, password: &QString) -> bool;

        #[qinvokable]
        fn edit(
            self: Pin<&mut Passwords>,
            id: i32,
            username: &QString,
            password: &QString,
            note: &QString,
        ) -> bool;

        #[qinvokable]
        fn remove(self: Pin<&mut Passwords>, id: i32) -> bool;

        /// Note that login `id` was just filled in.
        #[qinvokable]
        #[cxx_name = "markUsed"]
        fn mark_used(self: Pin<&mut Passwords>, id: i32);

        /// Never offer to save on the site at `url`.
        #[qinvokable]
        #[cxx_name = "neverSave"]
        fn never_save(self: Pin<&mut Passwords>, url: &QString);

        /// Origins on the never-save list, as a JSON array of
        /// `{origin, host}`.
        #[qinvokable]
        #[cxx_name = "neverSaved"]
        fn never_saved(self: Pin<&mut Passwords>) -> QString;

        #[qinvokable]
        #[cxx_name = "allowSave"]
        fn allow_save(self: Pin<&mut Passwords>, origin: &QString);

        /// Saved logins whose site, username or note contains `query`, as a
        /// JSON array of `{id, origin, host, username, note, created,
        /// changed, used, timesUsed, weak, reused}`, sorted by site.
        #[qinvokable]
        fn list(self: Pin<&mut Passwords>, query: &QString) -> QString;

        /// `{total, weak, reused}` counts for the checkup, as JSON.
        #[qinvokable]
        fn checkup(self: Pin<&mut Passwords>) -> QString;

        /// Import a CSV export (Chrome, Firefox, Safari, Bitwarden,
        /// 1Password…) at `path`, a file path or `file:` URL. Returns JSON
        /// `{added, updated, skipped}`, or `{error}`.
        #[qinvokable]
        #[cxx_name = "importFile"]
        fn import_file(self: Pin<&mut Passwords>, path: &QString) -> QString;

        /// Write every login as a CSV (Chrome's format) to `path`, readable
        /// only by the user. Returns "" or the error.
        #[qinvokable]
        #[cxx_name = "exportFile"]
        fn export_file(self: Pin<&mut Passwords>, path: &QString) -> QString;

        /// The login-form script for each tab's top frame
        /// (`ion_passwords::PAGE_SCRIPT`).
        #[qinvokable]
        #[cxx_name = "pageScript"]
        fn page_script(self: &Passwords) -> QString;

        /// Whether `a` and `b` are pages of the same origin, so a message a
        /// page sent about `a` still applies to the tab now showing `b`.
        #[qinvokable]
        #[cxx_name = "sameOrigin"]
        fn same_origin(self: &Passwords, a: &QString, b: &QString) -> bool;

        /// The site of `url` for showing to people ("github.com"), or "".
        #[qinvokable]
        fn host(self: &Passwords, url: &QString) -> QString;

        /// A new strong password.
        #[qinvokable]
        fn generate(self: &Passwords) -> QString;

        /// Copy `text` to the clipboard, hidden from clipboard history.
        #[qinvokable]
        #[cxx_name = "copySecret"]
        fn copy_secret_for(self: &Passwords, text: &QString);

        /// Clear the clipboard if it still holds `text`.
        #[qinvokable]
        #[cxx_name = "clearCopied"]
        fn clear_copied(self: &Passwords, text: &QString);

        /// Saved addresses containing `query`, most recently used first, as
        /// a JSON array of `ion_passwords::Address` plus a `summary`.
        #[qinvokable]
        fn addresses(self: Pin<&mut Passwords>, query: &QString) -> QString;

        /// Whether an address sent in a form (JSON `Address`) is worth
        /// offering to save: enough details, and not one already saved.
        #[qinvokable]
        #[cxx_name = "addressIsNew"]
        fn address_is_new(self: Pin<&mut Passwords>, address: &QString) -> bool;

        /// Save an address (JSON `Address`; id 0 adds one). Returns its id,
        /// or -1.
        #[qinvokable]
        #[cxx_name = "saveAddress"]
        fn save_address(self: Pin<&mut Passwords>, address: &QString) -> i32;

        #[qinvokable]
        #[cxx_name = "removeAddress"]
        fn remove_address(self: Pin<&mut Passwords>, id: i32) -> bool;

        /// Note that address `id` was just filled in.
        #[qinvokable]
        #[cxx_name = "markAddressUsed"]
        fn mark_address_used(self: Pin<&mut Passwords>, id: i32);
    }
}

use core::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use ion_passwords::{Address, Proposal, Vault, display_host, origin_of};
use serde_json::json;

#[derive(Default)]
pub struct PasswordsRust {
    revision: i32,
    vault: Option<Result<Vault, String>>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// A `file:` URL or a plain path.
fn local_path(path: &QString) -> std::path::PathBuf {
    let text = path.to_string();
    url::Url::parse(&text)
        .ok()
        .filter(|u| u.scheme() == "file")
        .and_then(|u| u.to_file_path().ok())
        .unwrap_or_else(|| text.into())
}

impl PasswordsRust {
    fn vault(&mut self) -> Result<&mut Vault, &str> {
        let opened = self.vault.get_or_insert_with(|| {
            let dir = ion_session::paths::data_dir().ok_or("no data folder")?;
            Vault::open(&dir, ion_keyring::global()).map_err(|err| {
                eprintln!("ion: could not open saved passwords: {err}");
                err.to_string()
            })
        });
        match opened {
            Ok(vault) => Ok(vault),
            Err(err) => Err(err.as_str()),
        }
    }
}

impl qobject::Passwords {
    fn changed(mut self: Pin<&mut Self>) {
        let revision = self.revision.wrapping_add(1);
        self.as_mut().rust_mut().revision = revision;
        self.revision_changed();
    }

    /// Run `f` on the vault, logging failures; `fallback` if it isn't open.
    fn with_vault<T>(
        mut self: Pin<&mut Self>,
        fallback: T,
        f: impl FnOnce(&mut Vault) -> ion_passwords::Result<T>,
    ) -> T {
        let mut this = self.as_mut().rust_mut();
        let Ok(vault) = this.vault() else {
            return fallback;
        };
        match f(vault) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("ion: saved passwords: {err}");
                fallback
            }
        }
    }

    fn enabled(&self) -> bool {
        ion_config::global().config().passwords.enable
    }

    fn problem(self: Pin<&mut Self>) -> QString {
        match self.rust_mut().vault() {
            Ok(_) => QString::default(),
            Err(err) => QString::from(err),
        }
    }

    fn store_name(&self) -> QString {
        QString::from(ion_keyring::global().backend().label())
    }

    fn store_is_keyring(&self) -> bool {
        ion_keyring::global().backend().is_os_keyring()
    }

    fn matches(self: Pin<&mut Self>, url: &QString) -> QString {
        let url = url.to_string();
        // Pages that can't have saved logins don't open the vault.
        if origin_of(&url).is_none() {
            return QString::from("[]");
        }
        let json = self.with_vault(String::from("[]"), |vault| {
            let list: Vec<_> = vault
                .matches(&url)
                .iter()
                .map(|m| {
                    json!({
                        "id": m.login.id,
                        "username": m.login.username,
                        "host": display_host(&m.login.origin),
                        "exact": m.fit == ion_passwords::Fit::Exact,
                    })
                })
                .collect();
            Ok(serde_json::Value::from(list).to_string())
        });
        QString::from(&json)
    }

    fn autofill_for(self: Pin<&mut Self>, url: &QString) -> i32 {
        let config = ion_config::global().config();
        if !config.passwords.autofill {
            return -1;
        }
        let url = url.to_string();
        let secure = url::Url::parse(&url).is_ok_and(|u| {
            u.scheme() == "https"
                || matches!(u.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
        });
        if !secure {
            return -1;
        }
        self.with_vault(-1, |vault| {
            Ok(vault
                .autofill_candidate(&url)
                .and_then(|l| i32::try_from(l.id).ok())
                .unwrap_or(-1))
        })
    }

    fn credentials(self: Pin<&mut Self>, id: i32) -> QString {
        let json = self.with_vault(String::new(), |vault| {
            Ok(u64::try_from(id)
                .ok()
                .and_then(|id| vault.get(id))
                .map(|l| {
                    json!({"id": l.id, "username": l.username, "password": l.password}).to_string()
                })
                .unwrap_or_default())
        });
        QString::from(&json)
    }

    fn propose(
        self: Pin<&mut Self>,
        url: &QString,
        username: &QString,
        password: &QString,
    ) -> QString {
        let nothing = || json!({"kind": "", "id": -1, "username": ""}).to_string();
        if !self.enabled() {
            return QString::from(&nothing());
        }
        let (url, username, password) =
            (url.to_string(), username.to_string(), password.to_string());
        if origin_of(&url).is_none() {
            return QString::from(&nothing());
        }
        let json = self.with_vault(nothing(), |vault| {
            Ok(match vault.propose(&url, &username, &password) {
                Proposal::Nothing => nothing(),
                Proposal::Save => {
                    json!({"kind": "save", "id": -1, "username": username}).to_string()
                }
                Proposal::Update(id) => {
                    let saved = vault
                        .get(id)
                        .map(|l| l.username.clone())
                        .unwrap_or_default();
                    json!({"kind": "update", "id": id, "username": saved}).to_string()
                }
            })
        });
        QString::from(&json)
    }

    fn save(
        mut self: Pin<&mut Self>,
        url: &QString,
        username: &QString,
        password: &QString,
    ) -> i32 {
        let (url, username, password) =
            (url.to_string(), username.to_string(), password.to_string());
        let id = self.as_mut().with_vault(-1, |vault| {
            let id = vault.save(&url, &username, &password, now_ms())?;
            Ok(i32::try_from(id).unwrap_or(-1))
        });
        self.changed();
        id
    }

    fn update_password(mut self: Pin<&mut Self>, id: i32, password: &QString) -> bool {
        let Ok(id) = u64::try_from(id) else {
            return false;
        };
        let password = password.to_string();
        let done = self
            .as_mut()
            .with_vault(false, |vault| vault.set_password(id, &password, now_ms()));
        self.changed();
        done
    }

    fn edit(
        mut self: Pin<&mut Self>,
        id: i32,
        username: &QString,
        password: &QString,
        note: &QString,
    ) -> bool {
        let Ok(id) = u64::try_from(id) else {
            return false;
        };
        let (username, password, note) =
            (username.to_string(), password.to_string(), note.to_string());
        let done = self.as_mut().with_vault(false, |vault| {
            vault.edit(id, &username, &password, &note, now_ms())
        });
        self.changed();
        done
    }

    fn remove(mut self: Pin<&mut Self>, id: i32) -> bool {
        let Ok(id) = u64::try_from(id) else {
            return false;
        };
        let done = self.as_mut().with_vault(false, |vault| vault.remove(id));
        self.changed();
        done
    }

    fn mark_used(self: Pin<&mut Self>, id: i32) {
        if let Ok(id) = u64::try_from(id) {
            self.with_vault((), |vault| vault.mark_used(id, now_ms()));
        }
    }

    fn never_save(mut self: Pin<&mut Self>, url: &QString) {
        let url = url.to_string();
        self.as_mut().with_vault((), |vault| vault.never_save(&url));
        self.changed();
    }

    fn never_saved(self: Pin<&mut Self>) -> QString {
        let json = self.with_vault(String::from("[]"), |vault| {
            let list: Vec<_> = vault
                .never_saved()
                .map(|origin| json!({"origin": origin, "host": display_host(origin)}))
                .collect();
            Ok(serde_json::Value::from(list).to_string())
        });
        QString::from(&json)
    }

    fn allow_save(mut self: Pin<&mut Self>, origin: &QString) {
        let origin = origin.to_string();
        self.as_mut()
            .with_vault((), |vault| vault.allow_save(&origin));
        self.changed();
    }

    fn list(self: Pin<&mut Self>, query: &QString) -> QString {
        let query = query.to_string().to_lowercase();
        let json = self.with_vault(String::from("[]"), |vault| {
            let checkup = vault.checkup();
            let mut list: Vec<_> = vault
                .logins()
                .iter()
                .filter(|l| {
                    query.is_empty()
                        || l.origin.to_lowercase().contains(&query)
                        || l.username.to_lowercase().contains(&query)
                        || l.note.to_lowercase().contains(&query)
                })
                .map(|l| {
                    let host = display_host(&l.origin);
                    (
                        host.clone(),
                        l.username.to_lowercase(),
                        json!({
                            "id": l.id,
                            "origin": l.origin,
                            "host": host,
                            "username": l.username,
                            "note": l.note,
                            "created": l.created,
                            "changed": l.changed,
                            "used": l.used,
                            "timesUsed": l.times_used,
                            "weak": checkup.weak.contains(&l.id),
                            "reused": checkup.reused.contains(&l.id),
                        }),
                    )
                })
                .collect();
            list.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
            let list: Vec<_> = list.into_iter().map(|(_, _, v)| v).collect();
            Ok(serde_json::Value::from(list).to_string())
        });
        QString::from(&json)
    }

    fn checkup(self: Pin<&mut Self>) -> QString {
        let json = self.with_vault(String::from("{}"), |vault| {
            let checkup = vault.checkup();
            Ok(json!({
                "total": vault.logins().len(),
                "weak": checkup.weak.len(),
                "reused": checkup.reused.len(),
            })
            .to_string())
        });
        QString::from(&json)
    }

    fn import_file(mut self: Pin<&mut Self>, path: &QString) -> QString {
        let path = local_path(path);
        let text = match std::fs::read(&path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(err) => return QString::from(&json!({"error": err.to_string()}).to_string()),
        };
        let result = match self.as_mut().rust_mut().vault() {
            Ok(vault) => vault.import_csv(&text, now_ms()).map_err(|e| e.to_string()),
            Err(err) => Err(err.to_owned()),
        };
        self.changed();
        QString::from(
            &match result {
                Ok(report) => json!({
                    "added": report.added,
                    "updated": report.updated,
                    "skipped": report.skipped,
                }),
                Err(err) => json!({"error": err}),
            }
            .to_string(),
        )
    }

    fn export_file(mut self: Pin<&mut Self>, path: &QString) -> QString {
        let path = local_path(path);
        let mut this = self.as_mut().rust_mut();
        let result = match this.vault() {
            Ok(vault) => ion_keyring::write_private(&path, vault.export_csv().as_bytes())
                .map_err(|e| e.to_string()),
            Err(err) => Err(err.to_owned()),
        };
        QString::from(&result.err().unwrap_or_default())
    }

    fn page_script(&self) -> QString {
        QString::from(ion_passwords::PAGE_SCRIPT)
    }

    fn same_origin(&self, a: &QString, b: &QString) -> bool {
        let a = origin_of(&a.to_string());
        a.is_some() && a == origin_of(&b.to_string())
    }

    fn host(&self, url: &QString) -> QString {
        QString::from(
            &origin_of(&url.to_string())
                .map(|o| display_host(&o))
                .unwrap_or_default(),
        )
    }

    fn generate(&self) -> QString {
        QString::from(&ion_passwords::generate(ion_passwords::DEFAULT_LENGTH))
    }

    fn copy_secret_for(&self, text: &QString) {
        qobject::copy_secret(text);
    }

    fn clear_copied(&self, text: &QString) {
        qobject::clear_secret(text);
    }

    fn addresses(self: Pin<&mut Self>, query: &QString) -> QString {
        let query = query.to_string().to_lowercase();
        let json = self.with_vault(String::from("[]"), |vault| {
            let list: Vec<_> = vault
                .addresses()
                .into_iter()
                .filter(|a| {
                    query.is_empty()
                        || serde_json::to_string(a)
                            .unwrap_or_default()
                            .to_lowercase()
                            .contains(&query)
                })
                .map(|a| {
                    let mut value = serde_json::to_value(a).unwrap_or_default();
                    value["summary"] = a.summary().into();
                    value
                })
                .collect();
            Ok(serde_json::Value::from(list).to_string())
        });
        QString::from(&json)
    }

    fn address_is_new(self: Pin<&mut Self>, address: &QString) -> bool {
        if !self.enabled() {
            return false;
        }
        let Ok(address) = serde_json::from_str::<Address>(&address.to_string()) else {
            return false;
        };
        self.with_vault(false, |vault| Ok(vault.address_is_new(&address)))
    }

    fn save_address(mut self: Pin<&mut Self>, address: &QString) -> i32 {
        let Ok(address) = serde_json::from_str::<Address>(&address.to_string()) else {
            return -1;
        };
        let id = self.as_mut().with_vault(-1, |vault| {
            Ok(i32::try_from(vault.save_address(&address)?).unwrap_or(-1))
        });
        self.changed();
        id
    }

    fn remove_address(mut self: Pin<&mut Self>, id: i32) -> bool {
        let Ok(id) = u64::try_from(id) else {
            return false;
        };
        let done = self
            .as_mut()
            .with_vault(false, |vault| vault.remove_address(id));
        self.changed();
        done
    }

    fn mark_address_used(self: Pin<&mut Self>, id: i32) {
        if let Ok(id) = u64::try_from(id) {
            self.with_vault((), |vault| vault.mark_address_used(id, now_ms()));
        }
    }
}
