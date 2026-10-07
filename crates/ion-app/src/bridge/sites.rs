//! `Sites` QML singleton: per-site settings from `[sites]` in config, and the
//! user scripts Ion injects into pages (per-site CSS plus the files in
//! `<config dir>/userscripts/`), all from `ion_sites`.
//!
//! QML installs `scripts()` on the profile and re-installs it when the config
//! changes or `revision` does (`reload()`, after editing files in the
//! userscripts folder).

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(i32, revision, READ, NOTIFY)]
        #[qproperty(QString, directory, READ, CONSTANT)]
        #[namespace = "ion"]
        type Sites = super::SitesRust;

        /// Whether pages at `url` may run JavaScript.
        #[qinvokable]
        #[cxx_name = "javascriptEnabled"]
        fn javascript_enabled(self: &Sites, url: &QString) -> bool;

        /// The scripts to install on the profile, as a list of
        /// `{ name, sourceCode, injectionPoint, mainWorld }` objects;
        /// `injectionPoint` is a `WebEngineScript.InjectionPoint` name.
        #[qinvokable]
        fn scripts(self: &Sites) -> QVariant;

        /// Re-read the userscripts folder (bumps `revision`).
        #[qinvokable]
        fn reload(self: Pin<&mut Sites>);

        /// Create the userscripts folder if needed. False if it can't be.
        #[qinvokable]
        #[cxx_name = "ensureDirectory"]
        fn ensure_directory(self: &Sites) -> bool;
    }
}

use core::pin::Pin;
use std::path::PathBuf;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QVariant};

pub struct SitesRust {
    revision: i32,
    directory: QString,
    dir: Option<PathBuf>,
}

impl Default for SitesRust {
    fn default() -> Self {
        let dir = ion_config::global()
            .paths()
            .map(|p| p.dir.join("userscripts"));
        let directory = dir
            .as_ref()
            .map(|d| QString::from(d.to_string_lossy().as_ref()))
            .unwrap_or_default();
        Self {
            revision: 0,
            directory,
            dir,
        }
    }
}

impl qobject::Sites {
    fn javascript_enabled(&self, url: &QString) -> bool {
        let config = ion_config::global().config();
        ion_sites::javascript_enabled(&config.sites, &url.to_string())
    }

    fn scripts(&self) -> QVariant {
        let config = ion_config::global().config();
        let (scripts, warnings) = ion_sites::all_scripts(&config, self.dir.as_deref());
        for warning in warnings {
            eprintln!("ion: userscripts: {warning}");
        }
        let mut list = QList::<QVariant>::default();
        for script in scripts {
            let mut row = QMap::<QMapPair_QString_QVariant>::default();
            let mut set = |key: &str, value: QVariant| row.insert(QString::from(key), value);
            set("name", QVariant::from(&QString::from(script.name.as_str())));
            set(
                "sourceCode",
                QVariant::from(&QString::from(script.source.as_str())),
            );
            set(
                "injectionPoint",
                QVariant::from(&QString::from(script.run_at.as_str())),
            );
            set(
                "mainWorld",
                QVariant::from(&(script.world == ion_sites::World::Main)),
            );
            list.append(QVariant::from(&row));
        }
        QVariant::from(&list)
    }

    fn ensure_directory(&self) -> bool {
        self.dir
            .as_deref()
            .is_some_and(|d| std::fs::create_dir_all(d).is_ok())
    }

    fn reload(mut self: Pin<&mut Self>) {
        let revision = self.revision.wrapping_add(1);
        self.as_mut().rust_mut().revision = revision;
        self.revision_changed();
    }
}
