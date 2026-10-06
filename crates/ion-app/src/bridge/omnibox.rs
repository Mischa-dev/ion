//! `Omnibox` QML element: resolves URL-bar input using `ion_core::navigation`,
//! with `!bangs` from `ion_bangs` as the first step.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, search_engine_name, cxx_name = "searchEngineName")]
        #[qproperty(QString, search_template, cxx_name = "searchTemplate")]
        #[namespace = "ion"]
        type Omnibox = super::OmniboxRust;

        /// Turn typed input into a URL to load. Returns an empty URL for blank input.
        #[qinvokable]
        fn resolve(self: &Omnibox, input: &QString) -> QUrl;

        /// True if `input` would run a search rather than open an address.
        #[qinvokable]
        #[cxx_name = "isSearch"]
        fn is_search(self: &Omnibox, input: &QString) -> bool;
    }
}

use std::sync::Arc;

use cxx_qt_lib::{QString, QUrl};
use ion_bangs::BangTable;
use ion_core::navigation::{Omnibox, Resolved, SearchEngine};

pub struct OmniboxRust {
    search_engine_name: QString,
    search_template: QString,
    bangs: Arc<BangTable>,
}

impl Default for OmniboxRust {
    fn default() -> Self {
        let engine = SearchEngine::default();
        Self {
            search_engine_name: QString::from(engine.name.as_str()),
            search_template: QString::from(engine.template.as_str()),
            bangs: Arc::new(BangTable::default()),
        }
    }
}

impl OmniboxRust {
    fn core(&self) -> Omnibox {
        Omnibox::new(SearchEngine::new(
            self.search_engine_name.to_string(),
            self.search_template.to_string(),
        ))
        .with_step(self.bangs.clone())
    }
}

impl qobject::Omnibox {
    fn resolve(&self, input: &QString) -> QUrl {
        match self.core().resolve(&input.to_string()) {
            Some(resolved) => QUrl::from(resolved.url()),
            None => QUrl::default(),
        }
    }

    fn is_search(&self, input: &QString) -> bool {
        matches!(
            self.core().resolve(&input.to_string()),
            Some(Resolved::Search(_))
        )
    }
}
