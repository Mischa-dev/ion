//! `Privacy` QML singleton: applies `[privacy]` from config to the browser
//! profile and clears browsing data on request.
//!
//! The Global Privacy Control header is added by the profile's request
//! interceptor (`bridge/adblock.rs`), and `navigator.globalPrivacyControl` by
//! a user script from `ion_sites`; this object handles cookies.

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

        /// Delete every cookie in `profile`, signing out of all sites.
        #[qinvokable]
        #[cxx_name = "clearCookies"]
        unsafe fn clear_cookies(self: &Privacy, profile: *mut QObject) -> bool;
    }
}

#[derive(Default)]
pub struct PrivacyRust;

/// With `history` in `[privacy] clearOnExit`, delete the engine's record of
/// visited links (which colors links as visited) for the "Default" profile.
/// QML has no call for it, so this removes the file before the engine opens
/// it; call once the application name is set and before QML loads.
pub fn forget_visited_links() {
    let config = ion_config::global().config();
    if !config
        .privacy
        .clear_on_exit
        .contains(&ion_config::BrowsingData::History)
    {
        return;
    }
    let dir = qobject::profile_storage_path(&cxx_qt_lib::QString::from("Default")).to_string();
    let file = std::path::Path::new(&dir).join("Visited Links");
    if let Err(err) = std::fs::remove_file(&file) {
        if err.kind() != std::io::ErrorKind::NotFound {
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
