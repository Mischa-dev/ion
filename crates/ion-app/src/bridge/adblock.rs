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
        // Follows `adblock.enable` in the config; QML switches it with `Config.set`.
        #[qproperty(bool, enabled, READ, NOTIFY)]
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
use std::sync::{Arc, Mutex};
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
    /// The lists `adblock.lists` names; shared with the background refresher.
    lists: Arc<Mutex<Vec<FilterList>>>,
    /// `adblock.lists` entries that are neither a known name nor a URL.
    unknown_lists: Vec<String>,
    /// Why the last list refresh failed, if it did.
    refresh_error: String,
    cache_dir: Option<PathBuf>,
    sites_path: Option<PathBuf>,
    attached: bool,
    config_subscription: Option<ion_config::Subscription>,
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
            lists: Arc::default(),
            unknown_lists: Vec::new(),
            refresh_error: String::new(),
            cache_dir: Store::default_dir(),
            sites_path,
            attached: false,
            config_subscription: None,
        }
    }
}

fn millis(time: Option<SystemTime>) -> f64 {
    time.and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0.0, |d| d.as_millis() as f64)
}

/// Outcome of one round of the worker, applied on the UI thread.
struct Refreshed {
    /// The lists this round used; its blocker is dropped if they changed since.
    lists: Vec<FilterList>,
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
        lists: lists.to_vec(),
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

        self.as_mut().apply_config();
        let thread = self.qt_thread();
        let subscription = ion_config::global().subscribe(move |_| {
            let _ = thread.queue(|obj| obj.apply_config());
        });
        self.as_mut().rust_mut().config_subscription = Some(subscription);

        let Some(cache_dir) = self.cache_dir.clone() else {
            self.as_mut()
                .set_error(QString::from("No cache directory for filter lists"));
            return true;
        };
        let lists = self.current_lists();

        // The compiled engine loads in milliseconds, so the first page is
        // already filtered. Compiling from scratch happens on the worker.
        let store = Store::new(&cache_dir);
        if let Some(blocker) = store.read_engine(&lists) {
            self.as_mut().install(blocker);
        }
        self.as_mut()
            .set_last_updated(millis(store.last_updated(&lists)));

        let have_blocker = self.shield.is_ready();
        let shared_lists = self.lists.clone();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let mut have_blocker = have_blocker;
            loop {
                if thread.queue(|obj| obj.set_updating(true)).is_err() {
                    return;
                }
                let lists = shared_lists.lock().map(|l| l.clone()).unwrap_or_default();
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

    fn current_lists(&self) -> Vec<FilterList> {
        self.lists.lock().map(|l| l.clone()).unwrap_or_default()
    }

    /// Take `adblock.enable` and `adblock.lists` from the config. New lists
    /// are downloaded and compiled in the background.
    fn apply_config(mut self: Pin<&mut Self>) {
        let config = ion_config::global().config();
        let enabled = config.adblock.enable;
        if self.enabled != enabled {
            let mut rust = self.as_mut().rust_mut();
            rust.enabled = enabled;
            rust.shield.set_enabled(enabled);
            self.as_mut().enabled_changed();
        }

        let (lists, unknown) = ion_adblock::lists::resolve(&config.adblock.lists);
        self.as_mut().rust_mut().unknown_lists = unknown;
        let changed = {
            let mut current = match self.lists.lock() {
                Ok(current) => current,
                Err(poisoned) => poisoned.into_inner(),
            };
            let changed = *current != lists;
            *current = lists;
            changed
        };
        // The first call, from `attach`, runs before anything is loaded.
        if changed && self.ready {
            self.as_mut().spawn_refresh(false, false);
        }
        self.as_mut().show_error();
    }

    /// Show the last refresh error and any unknown list names in `error`.
    fn show_error(mut self: Pin<&mut Self>) {
        let mut text = self.refresh_error.clone();
        if !self.unknown_lists.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&format!(
                "Unknown filter lists in config: {}",
                self.unknown_lists.join(", ")
            ));
        }
        self.as_mut().set_error(QString::from(text.as_str()));
    }

    /// Refresh lists on a one-off worker thread; `have_blocker: false` makes
    /// it compile even when no list changed.
    fn spawn_refresh(mut self: Pin<&mut Self>, force: bool, have_blocker: bool) {
        let Some(cache_dir) = self.cache_dir.clone() else {
            return;
        };
        let lists = self.current_lists();
        self.as_mut().set_updating(true);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let refreshed = refresh(&cache_dir, &lists, force, have_blocker);
            let _ = thread.queue(move |obj| obj.finish_refresh(refreshed));
        });
    }

    fn install(mut self: Pin<&mut Self>, blocker: Blocker) {
        self.as_mut().rust_mut().shield.set_blocker(blocker);
        self.as_mut().set_ready(true);
    }

    fn finish_refresh(mut self: Pin<&mut Self>, refreshed: Refreshed) {
        self.as_mut().set_updating(false);
        if refreshed.lists != self.current_lists() {
            // The config changed while this round ran; a newer one is coming.
            return;
        }
        if let Some(blocker) = refreshed.blocker {
            self.as_mut().install(blocker);
        }
        if refreshed.last_updated > 0.0 {
            self.as_mut().set_last_updated(refreshed.last_updated);
        }
        self.as_mut().rust_mut().refresh_error = refreshed.error;
        self.as_mut().show_error();
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
        let have_blocker = self.shield.is_ready();
        self.as_mut().spawn_refresh(true, have_blocker);
    }
}
