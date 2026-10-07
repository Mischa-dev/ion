//! `Privacy` QML singleton: applies `[privacy]` from config to the browser
//! profile and clears browsing data on request.
//!
//! The Global Privacy Control header is added by the profile's request
//! interceptor (`bridge/adblock.rs`), and `navigator.globalPrivacyControl` by
//! a user script from `ion_sites`; this object handles cookies.

#[cxx_qt::bridge]
pub mod qobject {
    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/privacy.h");

        #[cxx_name = "setThirdPartyCookiesBlocked"]
        unsafe fn set_third_party_cookies_blocked(profile: *mut QObject, blocked: bool) -> bool;

        #[cxx_name = "deleteAllCookies"]
        unsafe fn delete_all_cookies(profile: *mut QObject) -> bool;
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

        /// Delete every cookie in `profile`, signing out of all sites.
        #[qinvokable]
        #[cxx_name = "clearCookies"]
        unsafe fn clear_cookies(self: &Privacy, profile: *mut QObject) -> bool;
    }
}

#[derive(Default)]
pub struct PrivacyRust;

impl qobject::Privacy {
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
