//! `Extensions` QML singleton: which Chrome extensions to load
//! (`ion_platform::extensions`), from `extensions` in config and Ion's own
//! extensions folder.
//!
//! QML loads `paths()` through the profile's `extensionManager` at startup;
//! the extension list and enable switches come straight from that manager.

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
        #[qproperty(QString, folder, READ, CONSTANT)]
        #[namespace = "ion"]
        type Extensions = super::ExtensionsRust;

        /// Extension folders to load now. Each call re-reads config and the
        /// folder; `problems()` then lists the ones skipped and why.
        #[qinvokable]
        fn paths(self: Pin<&mut Extensions>) -> QStringList;

        /// "path: reason" for each extension the last `paths()` skipped.
        #[qinvokable]
        fn problems(self: &Extensions) -> QStringList;

        /// Create the extensions folder if needed. False if it can't be.
        #[qinvokable]
        #[cxx_name = "ensureFolder"]
        fn ensure_folder(self: &Extensions) -> bool;
    }
}

use core::pin::Pin;
use std::path::PathBuf;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use ion_platform::extensions::{self, Found};

pub struct ExtensionsRust {
    folder: QString,
    dir: Option<PathBuf>,
    problems: Vec<String>,
}

impl Default for ExtensionsRust {
    fn default() -> Self {
        let dir = ion_session::paths::data_dir().map(|d| d.join("extensions"));
        let folder = dir
            .as_ref()
            .map(|d| QString::from(d.to_string_lossy().as_ref()))
            .unwrap_or_default();
        Self {
            folder,
            dir,
            problems: Vec::new(),
        }
    }
}

impl qobject::Extensions {
    fn paths(mut self: Pin<&mut Self>) -> QStringList {
        let config = ion_config::global().config();
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let found = extensions::discover(&config.extensions, self.dir.as_deref(), home.as_deref());
        let mut paths = Vec::new();
        let mut problems = Vec::new();
        for f in found {
            match f {
                Found::Load(path) => paths.push(QString::from(path.to_string_lossy().as_ref())),
                Found::Skip { path, reason } => {
                    let line = format!("{}: {reason}", path.display());
                    eprintln!("ion: extension skipped: {line}");
                    problems.push(line);
                }
            }
        }
        self.as_mut().rust_mut().problems = problems;
        paths.into_iter().collect()
    }

    fn problems(&self) -> QStringList {
        self.problems
            .iter()
            .map(|p| QString::from(p.as_str()))
            .collect()
    }

    fn ensure_folder(&self) -> bool {
        self.dir
            .as_deref()
            .is_some_and(|d| std::fs::create_dir_all(d).is_ok())
    }
}
