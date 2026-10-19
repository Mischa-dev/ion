//! `Protocols` QML singleton: links that open other apps (`mailto:`,
//! `zoommtg:`…), from `ion_sites::protocols`. `ExternalAppPrompt.qml` asks
//! before handing one to the desktop; "always allow" answers are kept in
//! `<data dir>/protocols.json`.

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
        type Protocols = super::ProtocolsRust;

        /// Whether opening `url` would leave the browser for another app:
        /// its scheme isn't one the engine loads, and no site the person
        /// accepted handles it.
        #[qinvokable]
        #[cxx_name = "isExternal"]
        fn is_external(self: &Protocols, url: &QString) -> bool;

        /// Record that the site at `origin` now handles `scheme` links.
        #[qinvokable]
        #[cxx_name = "setHandler"]
        fn set_handler(self: &Protocols, scheme: &QString, origin: &QString);

        /// `[question, always-allow label]` for a page at `origin` opening `url`.
        #[qinvokable]
        #[cxx_name = "openPrompt"]
        fn open_prompt(self: &Protocols, origin: &QString, url: &QString) -> QStringList;

        /// The question for a site asking to handle `scheme` links itself.
        #[qinvokable]
        #[cxx_name = "handlerPrompt"]
        fn handler_prompt(self: &Protocols, origin: &QString, scheme: &QString) -> QString;

        /// Whether pages at `origin` may open `url` without asking.
        #[qinvokable]
        fn allows(self: &Protocols, origin: &QString, url: &QString) -> bool;

        /// Let pages at `origin` open links like `url` without asking.
        #[qinvokable]
        fn allow(self: &Protocols, origin: &QString, url: &QString);

        /// Forget which apps pages at `origin` may open, and the links it handles.
        #[qinvokable]
        fn forget(self: &Protocols, origin: &QString);
    }
}

use std::sync::Mutex;

use cxx_qt_lib::{QString, QStringList};
use ion_sites::protocols::{self, Approvals};

pub struct ProtocolsRust {
    approvals: Mutex<Approvals>,
}

impl Default for ProtocolsRust {
    fn default() -> Self {
        let approvals = ion_session::paths::data_dir()
            .map(|dir| Approvals::load(dir.join("protocols.json")))
            .unwrap_or_default();
        Self {
            approvals: Mutex::new(approvals),
        }
    }
}

impl ProtocolsRust {
    fn approvals(&self) -> std::sync::MutexGuard<'_, Approvals> {
        self.approvals.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl qobject::Protocols {
    fn is_external(&self, url: &QString) -> bool {
        let url = url.to_string();
        protocols::is_external(&url) && !self.approvals().has_handler(&url)
    }

    fn set_handler(&self, scheme: &QString, origin: &QString) {
        if let Err(err) = self
            .approvals()
            .set_handler(&scheme.to_string(), &origin.to_string())
        {
            eprintln!("ion: could not save which apps sites may open: {err}");
        }
    }

    fn open_prompt(&self, origin: &QString, url: &QString) -> QStringList {
        let (question, always) = protocols::open_prompt(&origin.to_string(), &url.to_string());
        [question, always]
            .iter()
            .map(|line| QString::from(line.as_str()))
            .collect()
    }

    fn handler_prompt(&self, origin: &QString, scheme: &QString) -> QString {
        QString::from(protocols::handler_prompt(&origin.to_string(), &scheme.to_string()).as_str())
    }

    fn allows(&self, origin: &QString, url: &QString) -> bool {
        self.approvals()
            .allows(&origin.to_string(), &url.to_string())
    }

    fn allow(&self, origin: &QString, url: &QString) {
        if let Err(err) = self
            .approvals()
            .allow(&origin.to_string(), &url.to_string())
        {
            eprintln!("ion: could not save which apps sites may open: {err}");
        }
    }

    fn forget(&self, origin: &QString) {
        if let Err(err) = self.approvals().forget(&origin.to_string()) {
            eprintln!("ion: could not save which apps sites may open: {err}");
        }
    }
}
