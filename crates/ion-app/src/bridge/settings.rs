//! `Preferences` QML singleton: what the Settings page (`ion://settings`)
//! shows, from `ion_settings`, with each setting's current value.
//!
//! Values are changed through the `Config` singleton, which writes
//! `overrides.toml`; this object only describes the page.

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
        #[namespace = "ion"]
        type Preferences = super::PreferencesRust;

        /// The sections matching `query` (all of them when blank) as JSON,
        /// each setting with its `value`, `default`, `origin` and `base`.
        /// `themes` are the theme ids to offer (`Theme.themeIds`). Re-read it
        /// when `Config.revision` changes.
        #[qinvokable]
        fn page(self: &Preferences, query: &QString, themes: &QStringList) -> QString;

        /// A built-in theme's background, surface, accent and text colors,
        /// for previews; empty for "auto" and themes from files.
        #[qinvokable]
        #[cxx_name = "themeSwatches"]
        fn theme_swatches(self: &Preferences, id: &QString) -> QStringList;

        /// Remove every change made in Settings. Returns an error or "".
        #[qinvokable]
        #[cxx_name = "resetAll"]
        fn reset_all(self: &Preferences) -> QString;
    }
}

use cxx_qt_lib::{QString, QStringList};

#[derive(Default)]
pub struct PreferencesRust;

impl qobject::Preferences {
    fn page(&self, query: &QString, themes: &QStringList) -> QString {
        let ctx = ion_settings::Context {
            themes: themes.iter().map(ToString::to_string).collect(),
        };
        let json = ion_settings::page_json(&ctx, &query.to_string(), ion_config::global());
        QString::from(json.as_str())
    }

    fn theme_swatches(&self, id: &QString) -> QStringList {
        ion_theme::builtin::get(&id.to_string())
            .map(|p| {
                [p.background, p.surface, p.accent, p.text]
                    .iter()
                    .map(|c| QString::from(c.to_string().as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn reset_all(&self) -> QString {
        let result = ion_config::global().reset_all();
        QString::from(result.err().unwrap_or_default().as_str())
    }
}
