//! `LoadErrors` QML singleton: the text of Ion's own error page, from
//! `ion_core::load_error`.

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
        type LoadErrors = super::LoadErrorsRust;

        /// `[title, detail]` for a failed load of `url`, or an empty list
        /// when no error page should show. `domain` and `code` are
        /// `WebEngineLoadingInfo.errorDomain` and `errorCode`; `message` is
        /// its `errorString`.
        #[qinvokable]
        fn describe(
            self: &LoadErrors,
            domain: i32,
            code: i32,
            url: &QString,
            message: &QString,
        ) -> QStringList;
    }
}

use cxx_qt_lib::{QString, QStringList};
use ion_core::load_error::{self, ErrorDomain};

#[derive(Default)]
pub struct LoadErrorsRust;

impl qobject::LoadErrors {
    fn describe(&self, domain: i32, code: i32, url: &QString, message: &QString) -> QStringList {
        let error = load_error::describe(
            ErrorDomain::from_qt(domain),
            code,
            &url.to_string(),
            &message.to_string(),
        );
        error
            .into_iter()
            .flat_map(|e| [e.title, e.detail])
            .map(|line| QString::from(line.as_str()))
            .collect()
    }
}
