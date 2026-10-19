//! `History` QML singleton: pages visited, backed by `ion_session::History`.
//!
//! Web views report finished loads with `recordVisit`; the command palette asks
//! `search`, which returns a JSON array of `{url, title, visits, lastVisit,
//! frecency}` so QML can merge it with other result sources using `JSON.parse`.
//!
//! It also keeps the address bar's input history (`ion_omnibox::inputs`):
//! which page was picked after typing what. That is history too, so removing a
//! page or clearing history forgets it as well.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[namespace = "ion"]
        type History = super::HistoryRust;

        /// History changed and should be saved soon.
        #[qsignal]
        fn changed(self: Pin<&mut History>);

        /// Record a finished page load. Ignores internal and blank pages.
        #[qinvokable]
        #[cxx_name = "recordVisit"]
        fn record_visit(self: Pin<&mut History>, url: &QString, title: &QString);

        /// Update the title of a page already in history.
        #[qinvokable]
        #[cxx_name = "updateTitle"]
        fn update_title(self: Pin<&mut History>, url: &QString, title: &QString);

        /// Best matches for `query` as a JSON array, at most `limit` of them.
        #[qinvokable]
        fn search(self: &History, query: &QString, limit: i32) -> QString;

        /// Remember that the address bar's `input` led to `url` (titled
        /// `title`), so typing it again ranks that page first.
        #[qinvokable]
        #[cxx_name = "recordChoice"]
        fn record_choice(self: Pin<&mut History>, input: &QString, url: &QString, title: &QString);

        /// Pages picked before after typing text starting with `query`, as
        /// a JSON array of `{url, title, boost}`, strongest first.
        #[qinvokable]
        fn chosen(self: &History, query: &QString) -> QString;

        /// Pages matching `query`, newest first, as JSON like `search`.
        #[qinvokable]
        fn recent(self: &History, query: &QString, limit: i32) -> QString;

        #[qinvokable]
        fn remove(self: Pin<&mut History>, url: &QString) -> bool;

        /// Forget every page on `site` and its subdomains. Returns how many.
        #[qinvokable]
        #[cxx_name = "removeSite"]
        fn remove_site(self: Pin<&mut History>, site: &QString) -> i32;

        #[qinvokable]
        fn clear(self: Pin<&mut History>);

        /// Write history to disk.
        #[qinvokable]
        fn save(self: &History) -> bool;

        /// Merge another browser's history (one of `Bookmarks.importSources()`).
        /// Returns how many pages were new, or -1 if it couldn't be read.
        #[qinvokable]
        #[cxx_name = "importFrom"]
        fn import_from(self: Pin<&mut History>, browser: &QString) -> i32;
    }
}

use core::pin::Pin;
use std::path::PathBuf;

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use ion_omnibox::InputHistory;
use ion_session::history::{self, History};

pub struct HistoryRust {
    history: History,
    path: Option<PathBuf>,
    inputs: InputHistory,
    inputs_path: Option<PathBuf>,
}

impl Default for HistoryRust {
    fn default() -> Self {
        let path = History::default_path();
        let history = path
            .as_deref()
            .map(|p| {
                History::load(p).unwrap_or_else(|err| {
                    eprintln!("ion: could not read history: {err}");
                    History::new()
                })
            })
            .unwrap_or_default();
        let inputs_path = ion_session::paths::data_dir().map(|d| d.join("inputs.json"));
        let inputs = inputs_path
            .as_deref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|json| {
                InputHistory::from_json(&json).unwrap_or_else(|err| {
                    eprintln!("ion: could not read address bar history: {err}");
                    InputHistory::default()
                })
            })
            .unwrap_or_default();
        Self {
            history,
            path,
            inputs,
            inputs_path,
        }
    }
}

impl qobject::History {
    fn record_visit(mut self: Pin<&mut Self>, url: &QString, title: &QString) {
        let now = history::now();
        if self
            .as_mut()
            .rust_mut()
            .history
            .record_visit(&url.to_string(), &title.to_string(), now)
        {
            self.changed();
        }
    }

    fn update_title(mut self: Pin<&mut Self>, url: &QString, title: &QString) {
        if self
            .as_mut()
            .rust_mut()
            .history
            .set_title(&url.to_string(), &title.to_string())
        {
            self.changed();
        }
    }

    fn search(&self, query: &QString, limit: i32) -> QString {
        let limit = usize::try_from(limit).unwrap_or(0);
        to_json(
            self.history
                .search(&query.to_string(), limit, history::now()),
        )
    }

    fn recent(&self, query: &QString, limit: i32) -> QString {
        let limit = usize::try_from(limit).unwrap_or(0);
        to_json(self.history.recent(&query.to_string(), limit))
    }

    fn remove_site(mut self: Pin<&mut Self>, site: &QString) -> i32 {
        let removed = self
            .as_mut()
            .rust_mut()
            .history
            .remove_site(&site.to_string());
        // What the address bar learned about the site goes with it.
        let forgot = self
            .as_mut()
            .rust_mut()
            .inputs
            .forget_site(&site.to_string());
        super::address_bar::forget_site_searches(Some(&site.to_string()));
        if removed > 0 || forgot {
            self.changed();
        }
        i32::try_from(removed).unwrap_or(i32::MAX)
    }

    fn record_choice(mut self: Pin<&mut Self>, input: &QString, url: &QString, title: &QString) {
        let (input, url, title) = (input.to_string(), url.to_string(), title.to_string());
        if !history::is_recordable(&url) {
            return;
        }
        if self
            .as_mut()
            .rust_mut()
            .inputs
            .record(&input, &url, &title, history::now())
        {
            self.changed();
        }
    }

    fn chosen(&self, query: &QString) -> QString {
        let pages: Vec<serde_json::Value> = self
            .inputs
            .boosts(&query.to_string(), history::now())
            .into_iter()
            .map(|page| {
                serde_json::json!({
                    "url": page.url,
                    "title": page.title,
                    "boost": page.frecency,
                })
            })
            .collect();
        QString::from(serde_json::Value::Array(pages).to_string().as_str())
    }

    fn remove(mut self: Pin<&mut Self>, url: &QString) -> bool {
        let url = url.to_string();
        let forgot = self.as_mut().rust_mut().inputs.forget_url(&url);
        let removed = self.as_mut().rust_mut().history.remove(&url) || forgot;
        if removed {
            self.changed();
        }
        removed
    }

    fn clear(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().history.clear();
        self.as_mut().rust_mut().inputs.clear();
        super::address_bar::forget_site_searches(None);
        self.changed();
    }

    fn import_from(mut self: Pin<&mut Self>, browser: &QString) -> i32 {
        let Some(imported) = super::bookmarks::read_profile(&browser.to_string()) else {
            return -1;
        };
        let added = self
            .as_mut()
            .rust_mut()
            .history
            .merge(imported.history, history::now());
        if added > 0 {
            self.changed();
        }
        i32::try_from(added).unwrap_or(i32::MAX)
    }

    fn save(&self) -> bool {
        let Some(path) = &self.path else {
            return false;
        };
        let saved = self
            .history
            .save(path)
            .map_err(|err| eprintln!("ion: could not save history: {err}"))
            .is_ok();
        let inputs_saved = self.inputs_path.as_deref().is_none_or(|path| {
            ion_session::paths::write_atomic(path, self.inputs.to_json().as_bytes())
                .map_err(|err| eprintln!("ion: could not save address bar history: {err}"))
                .is_ok()
        });
        saved && inputs_saved
    }
}

fn to_json(entries: Vec<&history::HistoryEntry>) -> QString {
    let now = history::now();
    let hits: Vec<serde_json::Value> = entries
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "url": e.url,
                "title": e.title,
                "visits": e.visits,
                "lastVisit": e.last_visit,
                "frecency": e.frecency(now),
            })
        })
        .collect();
    QString::from(serde_json::Value::Array(hits).to_string().as_str())
}
