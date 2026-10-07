//! `PaletteSearch` QML element: ranks results for the command palette using
//! `ion_bangs::Palette`.

// `query` takes one argument per palette source; QML can't pass a struct.
#![allow(clippy::too_many_arguments)]

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[namespace = "ion"]
        type PaletteSearch = super::PaletteSearchRust;

        /// Results for `input` as a list of `{ kind, title, subtitle, hint,
        /// action, value }` objects, best first. `titles` and `urls` describe
        /// the open tabs in order and `current` is the active tab's index;
        /// `history` is `History.search()`'s JSON, best first, and `bookmarks`
        /// is `Bookmarks.search()`'s; `sessions` are saved session names.
        ///
        /// `action` is `tab` (value: tab index), `open` (value: URL),
        /// `session` (value: session name), `run` (value: command id) or
        /// `complete` (value: new palette input).
        #[qinvokable]
        fn query(
            self: &PaletteSearch,
            input: &QString,
            titles: &QStringList,
            urls: &QStringList,
            current: i32,
            history: &QString,
            bookmarks: &QString,
            sessions: &QStringList,
        ) -> QVariant;
    }
}

use std::sync::{Arc, Mutex};

use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QStringList, QVariant};
use ion_bangs::{Action, BangTable, Palette, Sources, TabEntry};
use ion_config::Config;
use ion_core::navigation::{Omnibox, SearchEngine};

/// Ion's built-in bangs plus the `[bangs]` table from config, rebuilt only
/// when the config changes. Shared by the URL bar and the palette.
pub fn current_bangs() -> Arc<BangTable> {
    static CACHE: Mutex<Option<(Arc<Config>, Arc<BangTable>)>> = Mutex::new(None);
    let config = ion_config::global().config();
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((seen, table)) = cache.as_ref() {
        if Arc::ptr_eq(seen, &config) {
            return table.clone();
        }
    }
    let mut table = BangTable::default();
    let entries = config.bangs.iter().map(|(k, v)| (k.as_str(), v.as_str()));
    for error in table.apply(entries) {
        eprintln!("ion: config: {error}");
    }
    let table = Arc::new(table);
    *cache = Some((config, table.clone()));
    table
}

#[derive(Default)]
pub struct PaletteSearchRust;

impl PaletteSearchRust {
    fn palette(&self) -> Palette {
        let config = ion_config::global().config();
        let bangs = current_bangs();
        let engine = SearchEngine::new(&config.search.engine, &config.search.template);
        Palette::new(Omnibox::new(engine).with_step(bangs.clone()), bangs)
    }
}

fn strings(list: &QStringList) -> Vec<String> {
    QList::<QString>::from(list)
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// Pages from `History.search()`'s or `Bookmarks.search()`'s JSON array of
/// `{ url, title, … }`.
fn history_entries(json: &str) -> Vec<TabEntry> {
    let text =
        |value: &serde_json::Value, key: &str| value[key].as_str().unwrap_or_default().to_owned();
    match serde_json::from_str::<Vec<serde_json::Value>>(json) {
        Ok(pages) => pages
            .iter()
            .map(|page| TabEntry {
                title: text(page, "title"),
                url: text(page, "url"),
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Pair up parallel title and URL lists.
fn entries(titles: &QStringList, urls: &QStringList) -> Vec<TabEntry> {
    strings(titles)
        .into_iter()
        .zip(strings(urls))
        .map(|(title, url)| TabEntry { title, url })
        .collect()
}

impl qobject::PaletteSearch {
    fn query(
        &self,
        input: &QString,
        titles: &QStringList,
        urls: &QStringList,
        current: i32,
        history: &QString,
        bookmarks: &QString,
        sessions: &QStringList,
    ) -> QVariant {
        let tabs = entries(titles, urls);
        let history = history_entries(&history.to_string());
        let bookmarks = history_entries(&bookmarks.to_string());
        let sessions = strings(sessions);
        let sources = Sources {
            tabs: &tabs,
            current_tab: usize::try_from(current).ok(),
            history: &history,
            bookmarks: &bookmarks,
            sessions: &sessions,
        };

        let mut rows = QList::<QVariant>::default();
        for item in self.palette().query(&input.to_string(), &sources) {
            let (action, value) = match &item.action {
                Action::SwitchTab(index) => {
                    ("tab", QVariant::from(&i32::try_from(*index).unwrap_or(-1)))
                }
                Action::Open(url) => ("open", QVariant::from(&QString::from(url.as_str()))),
                Action::OpenSession(name) => {
                    ("session", QVariant::from(&QString::from(name.as_str())))
                }
                Action::Run(id) => ("run", QVariant::from(&QString::from(*id))),
                Action::Complete(text) => {
                    ("complete", QVariant::from(&QString::from(text.as_str())))
                }
            };
            let mut row = QMap::<QMapPair_QString_QVariant>::default();
            let mut set = |key: &str, value: QVariant| row.insert(QString::from(key), value);
            set("kind", QVariant::from(&QString::from(item.kind.as_str())));
            set("title", QVariant::from(&QString::from(item.title.as_str())));
            set(
                "subtitle",
                QVariant::from(&QString::from(item.subtitle.as_str())),
            );
            set("hint", QVariant::from(&QString::from(item.hint.as_str())));
            set("action", QVariant::from(&QString::from(action)));
            set("value", value);
            rows.append(QVariant::from(&row));
        }
        QVariant::from(&rows)
    }
}
