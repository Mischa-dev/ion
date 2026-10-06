//! `Adblock` QML singleton: network blocking with `ion_adblock`.
//!
//! The object is itself the profile's `QWebEngineUrlRequestInterceptor`, so
//! every request QtWebEngine makes (on the UI thread) goes through
//! [`ion_adblock::Shield::decide`]. Filter lists are refreshed on a worker
//! thread and the compiled engine is handed back to the UI thread.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
        include!(<QtCore/QObject>);
        type QObject = cxx_qt::QObject;

        include!("ion-app/cpp/interceptor.h");
        type QWebEngineUrlRequestInterceptor;
        type QWebEngineUrlRequestInfo;
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        #[cxx_name = "attachUrlRequestInterceptor"]
        unsafe fn attach_url_request_interceptor(
            profile: *mut QObject,
            interceptor: *mut QWebEngineUrlRequestInterceptor,
        ) -> bool;
        #[cxx_name = "requestUrl"]
        fn request_url(info: &QWebEngineUrlRequestInfo) -> QString;
        #[cxx_name = "requestFirstPartyUrl"]
        fn request_first_party_url(info: &QWebEngineUrlRequestInfo) -> QString;
        #[cxx_name = "requestMethod"]
        fn request_method(info: &QWebEngineUrlRequestInfo) -> QString;
        #[cxx_name = "requestResourceType"]
        fn request_resource_type(info: &QWebEngineUrlRequestInfo) -> i32;
        #[cxx_name = "blockRequest"]
        fn block_request(info: Pin<&mut QWebEngineUrlRequestInfo>);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[base = QWebEngineUrlRequestInterceptor]
        #[qproperty(bool, enabled, READ, WRITE = set_enabled, NOTIFY)]
        #[qproperty(bool, ready)]
        #[qproperty(bool, updating)]
        #[qproperty(i32, total_blocked, cxx_name = "totalBlocked")]
        #[qproperty(f64, last_updated, cxx_name = "lastUpdated")]
        #[qproperty(QString, error)]
        #[qproperty(i32, revision)]
        #[namespace = "ion"]
        type Adblock = super::AdblockRust;
    }

    unsafe extern "RustQt" {
        /// Start blocking on `profile` (a QML `WebEngineProfile`): load the
        /// cached filters and refresh stale lists in the background.
        #[qinvokable]
        unsafe fn attach(self: Pin<&mut Adblock>, profile: *mut QObject) -> bool;

        #[cxx_override]
        #[cxx_name = "interceptRequest"]
        fn intercept_request(self: Pin<&mut Adblock>, info: Pin<&mut QWebEngineUrlRequestInfo>);
    }

    extern "RustQt" {
        /// Setter of the global `enabled` switch.
        #[cxx_name = "setEnabled"]
        fn set_enabled(self: Pin<&mut Adblock>, enabled: bool);

        /// Requests blocked on the page at `url`. Re-read when `revision` changes.
        #[qinvokable]
        #[cxx_name = "blockedOn"]
        fn blocked_on(self: &Adblock, url: &QUrl) -> i32;

        /// Whether blocking applies on `url`'s site.
        #[qinvokable]
        #[cxx_name = "isEnabledOn"]
        fn is_enabled_on(self: &Adblock, url: &QUrl) -> bool;

        /// Switch blocking on or off for `url`'s site; remembered across restarts.
        #[qinvokable]
        #[cxx_name = "setEnabledOn"]
        fn set_enabled_on(self: Pin<&mut Adblock>, url: &QUrl, enabled: bool);

        /// The site name shown for `url` ("example.com"); empty for pages without one.
        #[qinvokable]
        #[cxx_name = "siteOf"]
        fn site_of(self: &Adblock, url: &QUrl) -> QString;

        /// Download every filter list now, even if the cached copies are fresh.
        #[qinvokable]
        #[cxx_name = "updateLists"]
        fn update_lists(self: Pin<&mut Adblock>);
    }

    impl cxx_qt::Threading for Adblock {}
}

use std::path::PathBuf;
use std::pin::Pin;
use std::time::{Duration, SystemTime};

use cxx_qt::casting::Upcast;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};
use ion_adblock::update::{self, HttpFetch};
use ion_adblock::{Blocker, FilterList, RequestInfo, ResourceType, Shield, SiteSettings, Store};

/// How often the worker checks whether lists went stale while Ion runs.
const RECHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

pub struct AdblockRust {
    enabled: bool,
    ready: bool,
    updating: bool,
    total_blocked: i32,
    /// Milliseconds since the epoch of the newest list download; 0 if never.
    last_updated: f64,
    error: QString,
    revision: i32,

    shield: Shield,
    lists: Vec<FilterList>,
    cache_dir: Option<PathBuf>,
    sites_path: Option<PathBuf>,
    attached: bool,
}

impl Default for AdblockRust {
    fn default() -> Self {
        let sites_path = SiteSettings::default_path();
        let sites = sites_path
            .as_deref()
            .and_then(|p| SiteSettings::load(p).ok())
            .unwrap_or_default();
        Self {
            enabled: true,
            ready: false,
            updating: false,
            total_blocked: 0,
            last_updated: 0.0,
            error: QString::default(),
            revision: 0,
            shield: Shield::new(sites),
            lists: ion_adblock::lists::defaults(),
            cache_dir: Store::default_dir(),
            sites_path,
            attached: false,
        }
    }
}

fn millis(time: Option<SystemTime>) -> f64 {
    time.and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0.0, |d| d.as_millis() as f64)
}

/// Outcome of one round of the worker, applied on the UI thread.
struct Refreshed {
    blocker: Option<Blocker>,
    last_updated: f64,
    error: String,
}

/// Download stale (or, with `force`, all) lists and compile them if anything
/// changed or nothing is loaded yet.
fn refresh(
    cache_dir: &PathBuf,
    lists: &[FilterList],
    force: bool,
    have_blocker: bool,
) -> Refreshed {
    let store = Store::new(cache_dir);
    let report = update::refresh(
        &store,
        lists,
        &HttpFetch::default(),
        SystemTime::now(),
        force,
    );
    let blocker = if report.changed() || !have_blocker {
        update::build(&store, lists)
    } else {
        None
    };
    let error = match report.failed.as_slice() {
        [] => String::new(),
        [(id, e)] => format!("Couldn't update {id}: {e}"),
        [(id, e), rest @ ..] => format!("Couldn't update {id} and {} more: {e}", rest.len()),
    };
    Refreshed {
        blocker,
        last_updated: millis(store.last_updated(lists)),
        error,
    }
}

impl qobject::Adblock {
    unsafe fn attach(mut self: Pin<&mut Self>, profile: *mut qobject::QObject) -> bool {
        let interceptor: Pin<&mut qobject::QWebEngineUrlRequestInterceptor> =
            self.as_mut().upcast_pin();
        // SAFETY: both pointers are live QObjects; the profile does not take
        // ownership of the interceptor, which as a QML singleton outlives it.
        let ok = unsafe {
            qobject::attach_url_request_interceptor(profile, interceptor.get_unchecked_mut())
        };
        if !ok || self.attached {
            return ok;
        }
        self.as_mut().rust_mut().attached = true;

        let Some(cache_dir) = self.cache_dir.clone() else {
            self.as_mut()
                .set_error(QString::from("No cache directory for filter lists"));
            return true;
        };
        let lists = self.lists.clone();

        // The compiled engine loads in milliseconds, so the first page is
        // already filtered. Compiling from scratch happens on the worker.
        let store = Store::new(&cache_dir);
        if let Some(blocker) = store.read_engine(&lists) {
            self.as_mut().install(blocker);
        }
        self.as_mut()
            .set_last_updated(millis(store.last_updated(&lists)));

        let have_blocker = self.shield.is_ready();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let mut have_blocker = have_blocker;
            loop {
                if thread.queue(|obj| obj.set_updating(true)).is_err() {
                    return;
                }
                let refreshed = refresh(&cache_dir, &lists, false, have_blocker);
                have_blocker |= refreshed.blocker.is_some();
                if thread
                    .queue(move |obj| obj.finish_refresh(refreshed))
                    .is_err()
                {
                    return;
                }
                std::thread::sleep(RECHECK_EVERY);
            }
        });
        true
    }

    fn install(mut self: Pin<&mut Self>, blocker: Blocker) {
        self.as_mut().rust_mut().shield.set_blocker(blocker);
        self.as_mut().set_ready(true);
    }

    fn finish_refresh(mut self: Pin<&mut Self>, refreshed: Refreshed) {
        if let Some(blocker) = refreshed.blocker {
            self.as_mut().install(blocker);
        }
        if refreshed.last_updated > 0.0 {
            self.as_mut().set_last_updated(refreshed.last_updated);
        }
        self.as_mut()
            .set_error(QString::from(refreshed.error.as_str()));
        self.as_mut().set_updating(false);
    }

    fn set_enabled(mut self: Pin<&mut Self>, enabled: bool) {
        if self.enabled == enabled {
            return;
        }
        let mut rust = self.as_mut().rust_mut();
        rust.enabled = enabled;
        rust.shield.set_enabled(enabled);
        self.as_mut().enabled_changed();
    }

    fn intercept_request(
        mut self: Pin<&mut Self>,
        info: Pin<&mut qobject::QWebEngineUrlRequestInfo>,
    ) {
        let url = qobject::request_url(&info).to_string();
        let first_party = qobject::request_first_party_url(&info).to_string();
        let method = qobject::request_method(&info).to_string();
        let resource = ResourceType::from_webengine(qobject::request_resource_type(&info));
        let request = RequestInfo {
            url: &url,
            first_party: &first_party,
            resource,
            method: &method,
        };

        let blocked = self.as_mut().rust_mut().shield.decide(&request);
        if blocked {
            qobject::block_request(info);
            let total = i32::try_from(self.shield.total_blocked()).unwrap_or(i32::MAX);
            self.as_mut().set_total_blocked(total);
        }
        // A page load resets its counter, so the UI re-reads it then too.
        if blocked || resource == ResourceType::MainFrame {
            let next = self.revision.wrapping_add(1);
            self.as_mut().set_revision(next);
        }
    }

    fn blocked_on(&self, url: &QUrl) -> i32 {
        i32::try_from(self.shield.blocked_on(&url.to_string())).unwrap_or(i32::MAX)
    }

    fn is_enabled_on(&self, url: &QUrl) -> bool {
        self.shield.is_enabled_on(&url.to_string())
    }

    fn set_enabled_on(mut self: Pin<&mut Self>, url: &QUrl, enabled: bool) {
        let changed = self
            .as_mut()
            .rust_mut()
            .shield
            .set_enabled_on(&url.to_string(), enabled);
        if !changed {
            return;
        }
        let saved = match &self.sites_path {
            Some(path) => self.shield.sites().save(path),
            None => Ok(()),
        };
        if let Err(e) = saved {
            let message = format!("Couldn't save site settings: {e}");
            self.as_mut().set_error(QString::from(message.as_str()));
        }
        let next = self.revision.wrapping_add(1);
        self.as_mut().set_revision(next);
    }

    fn site_of(&self, url: &QUrl) -> QString {
        QString::from(
            ion_adblock::sites::site_of(&url.to_string())
                .unwrap_or_default()
                .as_str(),
        )
    }

    fn update_lists(mut self: Pin<&mut Self>) {
        if self.updating {
            return;
        }
        let Some(cache_dir) = self.cache_dir.clone() else {
            return;
        };
        let lists = self.lists.clone();
        let have_blocker = self.shield.is_ready();
        self.as_mut().set_updating(true);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let refreshed = refresh(&cache_dir, &lists, true, have_blocker);
            let _ = thread.queue(move |obj| obj.finish_refresh(refreshed));
        });
    }
}
