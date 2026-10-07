//! `History` QML singleton: pages visited, backed by `ion_session::History`.
//!
//! Web views report finished loads with `recordVisit`; the command palette asks
//! `search`, which returns a JSON array of `{url, title, visits, lastVisit}` so
//! QML can merge it with other result sources using `JSON.parse`.

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

        #[qinvokable]
        fn remove(self: Pin<&mut History>, url: &QString) -> bool;

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
use ion_session::history::{self, History};

pub struct HistoryRust {
    history: History,
    path: Option<PathBuf>,
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
        Self { history, path }
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
        let hits: Vec<serde_json::Value> = self
            .history
            .search(&query.to_string(), limit, history::now())
            .into_iter()
            .map(|e| {
                serde_json::json!({
                    "url": e.url,
                    "title": e.title,
                    "visits": e.visits,
                    "lastVisit": e.last_visit,
                })
            })
            .collect();
        QString::from(serde_json::Value::Array(hits).to_string().as_str())
    }

    fn remove(mut self: Pin<&mut Self>, url: &QString) -> bool {
        let removed = self.as_mut().rust_mut().history.remove(&url.to_string());
        if removed {
            self.changed();
        }
        removed
    }

    fn clear(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().history.clear();
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
        self.history
            .save(path)
            .map_err(|err| eprintln!("ion: could not save history: {err}"))
            .is_ok()
    }
}
