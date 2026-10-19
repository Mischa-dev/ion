//! `Privacy` QML singleton: applies `[privacy]` from config to the browser
//! profile and clears browsing data on request.
//!
//! The Global Privacy Control header is added by the profile's request
//! interceptor (`bridge/adblock.rs`), and `navigator.globalPrivacyControl` by
//! a user script from `ion_sites`; this object handles cookies, HTTPS-first
//! (`ion_privacy::https`, consulted by that same interceptor through
//! [`https_upgrade`]) and the fingerprinting-protection script.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/privacy.h");

        #[cxx_name = "setThirdPartyCookiesBlocked"]
        unsafe fn set_third_party_cookies_blocked(profile: *mut QObject, blocked: bool) -> bool;

        #[cxx_name = "deleteAllCookies"]
        unsafe fn delete_all_cookies(profile: *mut QObject) -> bool;

        #[cxx_name = "profileStoragePath"]
        fn profile_storage_path(storage_name: &QString) -> QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[namespace = "ion"]
        type Privacy = super::PrivacyRust;

        /// Apply the current `[privacy]` settings to `profile` (a QML
        /// `WebEngineProfile`). Call again after the config changes.
        #[qinvokable]
        unsafe fn apply(self: &Privacy, profile: *mut QObject) -> bool;

        /// What `[privacy] clearOnExit` asks to delete: some of `cookies`,
        /// `cache` and `history`.
        #[qinvokable]
        #[cxx_name = "clearOnExit"]
        fn clear_on_exit(self: &Privacy) -> QStringList;

        /// `privacy.httpsOnly`: "upgrade", "strict" or "off".
        #[qinvokable]
        #[cxx_name = "httpsMode"]
        fn https_mode(self: &Privacy) -> QString;

        /// After the https page at `url` failed to load: the http address
        /// to load instead if Ion upgraded it, else "". In "upgrade" mode the
        /// site stays on http for the session.
        #[qinvokable]
        #[cxx_name = "httpsFallback"]
        fn https_fallback(self: &Privacy, url: &QString) -> QString;

        /// Load `url`'s site over http for the rest of the session.
        #[qinvokable]
        #[cxx_name = "allowHttp"]
        fn allow_http(self: &Privacy, url: &QString);

        /// Whether `url`'s site was let through over http this session.
        #[qinvokable]
        #[cxx_name = "httpAllowed"]
        fn http_allowed(self: &Privacy, url: &QString) -> bool;

        /// The fingerprinting-protection script (empty when
        /// `privacy.fingerprinting` is off), skipping sites whose shield is
        /// switched off.
        #[qinvokable]
        #[cxx_name = "fingerprintScript"]
        fn fingerprint_script(self: &Privacy) -> QString;

        /// Whether `privacy.fingerprinting` is on.
        #[qinvokable]
        fn fingerprinting(self: &Privacy) -> bool;

        /// Whether `privacy.blockThirdPartyCookies` is on.
        #[qinvokable]
        #[cxx_name = "thirdPartyCookiesBlocked"]
        fn third_party_cookies_blocked(self: &Privacy) -> bool;

        /// Whether Global Privacy Control is sent.
        #[qinvokable]
        #[cxx_name = "globalPrivacyControl"]
        fn global_privacy_control(self: &Privacy) -> bool;

        /// "normal", "private" or "profile": what kind of window this is.
        #[qinvokable]
        #[cxx_name = "windowKind"]
        fn window_kind(self: &Privacy) -> QString;

        /// The profile's name ("Work"), "Private", or "" for the default.
        #[qinvokable]
        #[cxx_name = "profileName"]
        fn profile_name(self: &Privacy) -> QString;

        /// The browser profile's storage name: "" (off the record) in a
        /// private window.
        #[qinvokable]
        #[cxx_name = "storageName"]
        fn storage_name(self: &Privacy) -> QString;

        /// Open a private window, a separate process, with `urls` in it.
        #[qinvokable]
        #[cxx_name = "openPrivateWindow"]
        fn open_private_window(self: &Privacy, urls: &QStringList) -> bool;

        /// Open (or switch to) profile `name`, creating it if new. "" opens
        /// the default profile. False if the name isn't usable.
        #[qinvokable]
        #[cxx_name = "openProfile"]
        fn open_profile(self: &Privacy, name: &QString) -> bool;

        /// The profiles created so far, by name.
        #[qinvokable]
        fn profiles(self: &Privacy) -> QStringList;

        /// Delete every cookie in `profile`, signing out of all sites.
        #[qinvokable]
        #[cxx_name = "clearCookies"]
        unsafe fn clear_cookies(self: &Privacy, profile: *mut QObject) -> bool;
    }
}

#[derive(Default)]
pub struct PrivacyRust;

static HTTPS: std::sync::LazyLock<std::sync::Mutex<ion_privacy::Https>> =
    std::sync::LazyLock::new(Default::default);

fn https() -> std::sync::MutexGuard<'static, ion_privacy::Https> {
    HTTPS.lock().unwrap_or_else(|e| e.into_inner())
}

fn https_mode() -> ion_privacy::HttpsMode {
    ion_privacy::HttpsMode::parse(&ion_config::global().config().privacy.https_only)
}

/// The https address to load instead of the top-level navigation to `url`,
/// if HTTPS-first applies. Called by the request interceptor.
pub fn https_upgrade(url: &str) -> Option<String> {
    match https().upgrade(url, https_mode(), std::time::Instant::now()) {
        ion_privacy::Upgrade::To(target) => Some(target),
        ion_privacy::Upgrade::Keep => None,
    }
}

/// The default profile's data folder, also from inside another profile
/// (whose `ION_DATA_DIR` points at `<base>/profiles/<name>`).
fn base_data_dir() -> Option<std::path::PathBuf> {
    let dir = ion_session::paths::data_dir()?;
    Some(match ion_privacy::window::mode() {
        ion_privacy::window::Mode::Profile(_) => dir.parent()?.parent()?.to_path_buf(),
        _ => dir,
    })
}

/// Start another Ion window with `args`. A profile's folder is chosen by its
/// `--profile` argument, so the child starts from the default data folder.
fn spawn_window(args: &[String]) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let mut command = std::process::Command::new(exe);
    command.args(args);
    if let (ion_privacy::window::Mode::Profile(_), Some(base)) =
        (ion_privacy::window::mode(), base_data_dir())
    {
        command.env(ion_session::paths::DATA_DIR_ENV, base);
    }
    match command.spawn() {
        Ok(mut child) => {
            // Reap it when it quits, so it doesn't linger as a zombie.
            std::thread::spawn(move || child.wait());
            true
        }
        Err(err) => {
            eprintln!("ion: could not open a new window: {err}");
            false
        }
    }
}

/// A random number for this run, so fingerprinting noise differs between
/// sessions.
fn session_seed() -> u32 {
    static SEED: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *SEED.get_or_init(|| {
        let mut bytes = [0u8; 4];
        if getrandom::fill(&mut bytes).is_err() {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(1);
            bytes = nanos.to_le_bytes();
        }
        u32::from_le_bytes(bytes)
    })
}

/// Delete the "Default" profile's files for what `[privacy] clearOnExit`
/// lists, before the engine opens them: the cookie database for `cookies`
/// (so no page can see a cookie from before), and for `history` the engine's
/// own history, favicon and visited-links databases (QML has no call for
/// them). Call once the application name is
/// set and before QML loads.
pub fn clear_profile_files() {
    use ion_config::BrowsingData;
    let config = ion_config::global().config();
    let mut files = Vec::new();
    for data in &config.privacy.clear_on_exit {
        match data {
            BrowsingData::Cookies => files.extend(["Cookies", "Cookies-journal"]),
            BrowsingData::History => files.extend([
                "Visited Links",
                "History",
                "History-journal",
                "Favicons",
                "Favicons-journal",
            ]),
            BrowsingData::Cache => {}
        }
    }
    if files.is_empty() {
        return;
    }
    let storage = ion_privacy::window::mode().storage_name();
    // A private window keeps nothing on disk.
    if storage.is_empty() {
        return;
    }
    let dir = qobject::profile_storage_path(&cxx_qt_lib::QString::from(&storage)).to_string();
    for name in files {
        let file = std::path::Path::new(&dir).join(name);
        if let Err(err) = std::fs::remove_file(&file)
            && err.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!("ion: could not delete {}: {err}", file.display());
        }
    }
}

impl qobject::Privacy {
    fn clear_on_exit(&self) -> cxx_qt_lib::QStringList {
        let config = ion_config::global().config();
        config
            .privacy
            .clear_on_exit
            .iter()
            .map(|data| cxx_qt_lib::QString::from(data.as_str()))
            .collect()
    }

    fn window_kind(&self) -> cxx_qt_lib::QString {
        use ion_privacy::window::Mode;
        cxx_qt_lib::QString::from(match ion_privacy::window::mode() {
            Mode::Normal => "normal",
            Mode::Private => "private",
            Mode::Profile(_) => "profile",
        })
    }

    fn profile_name(&self) -> cxx_qt_lib::QString {
        cxx_qt_lib::QString::from(ion_privacy::window::mode().label())
    }

    fn storage_name(&self) -> cxx_qt_lib::QString {
        cxx_qt_lib::QString::from(&ion_privacy::window::mode().storage_name())
    }

    fn open_private_window(&self, urls: &cxx_qt_lib::QStringList) -> bool {
        let mut args = ion_privacy::window::Mode::Private.args();
        args.extend(urls.iter().map(|u| u.to_string()));
        spawn_window(&args)
    }

    fn open_profile(&self, name: &cxx_qt_lib::QString) -> bool {
        use ion_privacy::window::{Mode, clean_name};
        let name = name.to_string();
        let mode = if name.trim().is_empty() {
            Mode::Normal
        } else {
            match clean_name(&name) {
                Some(name) => Mode::Profile(name),
                None => return false,
            }
        };
        if let (Mode::Profile(_), Some(base)) = (&mode, base_data_dir())
            && let Err(err) = std::fs::create_dir_all(mode.data_dir(&base))
        {
            eprintln!("ion: could not create the profile folder: {err}");
            return false;
        }
        spawn_window(&mode.args())
    }

    fn profiles(&self) -> cxx_qt_lib::QStringList {
        base_data_dir()
            .map(|base| ion_privacy::window::profiles(&base))
            .unwrap_or_default()
            .iter()
            .map(|name| cxx_qt_lib::QString::from(name.as_str()))
            .collect()
    }

    fn https_mode(&self) -> cxx_qt_lib::QString {
        cxx_qt_lib::QString::from(&ion_config::global().config().privacy.https_only)
    }

    fn https_fallback(&self, url: &cxx_qt_lib::QString) -> cxx_qt_lib::QString {
        let target = https().fallback(&url.to_string(), https_mode());
        cxx_qt_lib::QString::from(&target.unwrap_or_default())
    }

    fn allow_http(&self, url: &cxx_qt_lib::QString) {
        https().allow_http(&url.to_string());
    }

    fn http_allowed(&self, url: &cxx_qt_lib::QString) -> bool {
        https().is_allowed(&url.to_string())
    }

    fn fingerprinting(&self) -> bool {
        ion_config::global().config().privacy.fingerprinting
    }

    fn third_party_cookies_blocked(&self) -> bool {
        ion_config::global()
            .config()
            .privacy
            .block_third_party_cookies
    }

    fn global_privacy_control(&self) -> bool {
        ion_config::global().config().privacy.global_privacy_control
    }

    fn fingerprint_script(&self) -> cxx_qt_lib::QString {
        if !self.fingerprinting() {
            return cxx_qt_lib::QString::default();
        }
        // Sites where the shield is off get no protection either, so one
        // switch fixes a site that breaks.
        let sites = ion_adblock::SiteSettings::default_path()
            .and_then(|path| ion_adblock::SiteSettings::load(&path).ok())
            .unwrap_or_default();
        cxx_qt_lib::QString::from(&ion_privacy::fingerprint_script(
            session_seed(),
            sites.disabled_sites(),
        ))
    }

    /// # Safety
    /// `profile` must be null or a live QObject.
    unsafe fn apply(&self, profile: *mut qobject::QObject) -> bool {
        if profile.is_null() {
            return false;
        }
        let config = ion_config::global().config();
        // SAFETY: `profile` is a live QObject (checked non-null above, owned by QML).
        unsafe {
            qobject::set_third_party_cookies_blocked(
                profile,
                config.privacy.block_third_party_cookies,
            )
        }
    }

    /// # Safety
    /// `profile` must be null or a live QObject.
    unsafe fn clear_cookies(&self, profile: *mut qobject::QObject) -> bool {
        if profile.is_null() {
            return false;
        }
        // SAFETY: as in `apply`.
        unsafe { qobject::delete_all_cookies(profile) }
    }
}
