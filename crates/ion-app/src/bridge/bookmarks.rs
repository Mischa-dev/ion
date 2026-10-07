//! `Bookmarks` QML singleton: bookmarked pages, backed by
//! `ion_bookmarks::Bookmarks`, plus importing bookmarks from other browsers.
//!
//! `revision` changes whenever the list does, so bindings such as the URL
//! bar's star can depend on it. Every change is saved right away; bookmarks
//! change rarely and losing one is worse than a small write.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(i32, revision, READ, NOTIFY)]
        #[qproperty(i32, count, READ, NOTIFY)]
        #[namespace = "ion"]
        type Bookmarks = super::BookmarksRust;

        #[qinvokable]
        fn contains(self: &Bookmarks, url: &QString) -> bool;

        /// Bookmark `url` or remove its bookmark. Returns whether it is
        /// bookmarked afterwards.
        #[qinvokable]
        fn toggle(self: Pin<&mut Bookmarks>, url: &QString, title: &QString) -> bool;

        #[qinvokable]
        fn remove(self: Pin<&mut Bookmarks>, url: &QString) -> bool;

        /// Best matches for `query` as a JSON array of `{url, title, folder}`,
        /// at most `limit` of them.
        #[qinvokable]
        fn search(self: &Bookmarks, query: &QString, limit: i32) -> QString;

        /// Browsers with a profile Ion can import from, e.g. "Firefox".
        #[qinvokable]
        #[cxx_name = "importSources"]
        fn import_sources(self: &Bookmarks) -> QStringList;

        /// Import `browser`'s bookmarks. Returns how many were new, or -1 if
        /// the profile couldn't be read.
        #[qinvokable]
        #[cxx_name = "importFrom"]
        fn import_from(self: Pin<&mut Bookmarks>, browser: &QString) -> i32;
    }
}

use core::pin::Pin;
use std::path::PathBuf;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use ion_bookmarks::{Bookmarks, import};

pub struct BookmarksRust {
    bookmarks: Bookmarks,
    path: Option<PathBuf>,
    revision: i32,
    count: i32,
}

impl Default for BookmarksRust {
    fn default() -> Self {
        let path = Bookmarks::default_path();
        let bookmarks = path
            .as_deref()
            .map(|p| {
                Bookmarks::load(p).unwrap_or_else(|err| {
                    eprintln!("ion: could not read bookmarks: {err}");
                    Bookmarks::new()
                })
            })
            .unwrap_or_default();
        let count = i32::try_from(bookmarks.len()).unwrap_or(i32::MAX);
        Self {
            bookmarks,
            path,
            revision: 0,
            count,
        }
    }
}

/// Profiles of other browsers installed for this user.
pub fn import_profiles() -> Vec<import::Profile> {
    std::env::var_os("HOME")
        .map(|home| import::find_profiles(&PathBuf::from(home)))
        .unwrap_or_default()
}

/// Read the profile of `browser` (as listed by [`import_profiles`]).
pub fn read_profile(browser: &str) -> Option<import::Imported> {
    let profile = import_profiles()
        .into_iter()
        .find(|p| p.browser == browser)?;
    import::import(&profile)
        .map_err(|err| eprintln!("ion: could not import from {browser}: {err}"))
        .ok()
}

impl qobject::Bookmarks {
    /// Save, then tell QML the list changed.
    fn changed(mut self: Pin<&mut Self>) {
        if let Some(path) = &self.path {
            if let Err(err) = self.bookmarks.save(path) {
                eprintln!("ion: could not save bookmarks: {err}");
            }
        }
        let revision = self.revision.wrapping_add(1);
        let count = i32::try_from(self.bookmarks.len()).unwrap_or(i32::MAX);
        let mut this = self.as_mut().rust_mut();
        this.revision = revision;
        let count_changed = this.count != count;
        this.count = count;
        if count_changed {
            self.as_mut().count_changed();
        }
        self.revision_changed();
    }

    fn contains(&self, url: &QString) -> bool {
        self.bookmarks.contains(&url.to_string())
    }

    fn toggle(mut self: Pin<&mut Self>, url: &QString, title: &QString) -> bool {
        let url = url.to_string();
        let was = self.bookmarks.contains(&url);
        let now = ion_session::history::now();
        let is = self
            .as_mut()
            .rust_mut()
            .bookmarks
            .toggle(&url, &title.to_string(), now);
        if was != is {
            self.changed();
        }
        is
    }

    fn remove(mut self: Pin<&mut Self>, url: &QString) -> bool {
        let removed = self.as_mut().rust_mut().bookmarks.remove(&url.to_string());
        if removed {
            self.changed();
        }
        removed
    }

    fn search(&self, query: &QString, limit: i32) -> QString {
        let limit = usize::try_from(limit).unwrap_or(0);
        let hits: Vec<serde_json::Value> = self
            .bookmarks
            .search(&query.to_string(), limit)
            .into_iter()
            .map(|b| serde_json::json!({ "url": b.url, "title": b.title, "folder": b.folder }))
            .collect();
        QString::from(serde_json::Value::Array(hits).to_string().as_str())
    }

    fn import_sources(&self) -> QStringList {
        import_profiles()
            .iter()
            .map(|p| QString::from(p.browser.as_str()))
            .collect()
    }

    fn import_from(mut self: Pin<&mut Self>, browser: &QString) -> i32 {
        let Some(imported) = read_profile(&browser.to_string()) else {
            return -1;
        };
        let added = self
            .as_mut()
            .rust_mut()
            .bookmarks
            .extend(imported.bookmarks);
        if added > 0 {
            self.changed();
        }
        i32::try_from(added).unwrap_or(i32::MAX)
    }
}
