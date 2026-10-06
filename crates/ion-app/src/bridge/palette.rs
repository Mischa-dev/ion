//! `PaletteSearch` QML element: ranks results for the command palette using
//! `ion_bangs::Palette`.

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
        /// the open tabs in order; `current` is the active tab's index.
        ///
        /// `action` is `tab` (value: tab index), `open` (value: URL), `run`
        /// (value: command id) or `complete` (value: new palette input).
        #[qinvokable]
        fn query(
            self: &PaletteSearch,
            input: &QString,
            titles: &QStringList,
            urls: &QStringList,
            current: i32,
        ) -> QVariant;
    }
}

use std::sync::Arc;

use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QStringList, QVariant};
use ion_bangs::{Action, BangTable, Palette, TabEntry};
use ion_core::navigation::Omnibox;

pub struct PaletteSearchRust {
    palette: Palette,
}

impl Default for PaletteSearchRust {
    fn default() -> Self {
        let bangs = Arc::new(BangTable::default());
        let omnibox = Omnibox::default().with_step(bangs.clone());
        Self {
            palette: Palette::new(omnibox, bangs),
        }
    }
}

fn strings(list: &QStringList) -> Vec<String> {
    QList::<QString>::from(list)
        .iter()
        .map(ToString::to_string)
        .collect()
}

impl qobject::PaletteSearch {
    fn query(
        &self,
        input: &QString,
        titles: &QStringList,
        urls: &QStringList,
        current: i32,
    ) -> QVariant {
        let mut urls = strings(urls).into_iter();
        let tabs: Vec<TabEntry> = strings(titles)
            .into_iter()
            .map(|title| TabEntry {
                title,
                url: urls.next().unwrap_or_default(),
            })
            .collect();
        let current = usize::try_from(current).ok();

        let mut rows = QList::<QVariant>::default();
        for item in self.palette.query(&input.to_string(), &tabs, current) {
            let (action, value) = match &item.action {
                Action::SwitchTab(index) => {
                    ("tab", QVariant::from(&i32::try_from(*index).unwrap_or(-1)))
                }
                Action::Open(url) => ("open", QVariant::from(&QString::from(url.as_str()))),
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
