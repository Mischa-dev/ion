//! `Pages` QML singleton: Ion's own pages at `ion://<page>`
//! (`ion_core::pages`).
//!
//! The object is the profile's handler for the `ion:` scheme. It serves each
//! page an empty, titled document so the page is a real history entry; the
//! QML page (BuiltinPage.qml) is drawn over the tab.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
        include!(<QtCore/QObject>);
        type QObject = cxx_qt::QObject;

        include!("ion-app/cpp/pages.h");
        type QWebEngineUrlSchemeHandler;
        type QWebEngineUrlRequestJob;
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        #[cxx_name = "registerIonScheme"]
        fn register_ion_scheme();
        #[cxx_name = "installIonSchemeHandler"]
        unsafe fn install_ion_scheme_handler(
            profile: *mut QObject,
            handler: *mut QWebEngineUrlSchemeHandler,
        ) -> bool;
        #[cxx_name = "jobUrl"]
        fn job_url(job: &QWebEngineUrlRequestJob) -> QString;
        #[cxx_name = "replyHtml"]
        fn reply_html(job: Pin<&mut QWebEngineUrlRequestJob>, html: &QString);
        #[cxx_name = "failJobNotFound"]
        fn fail_job_not_found(job: Pin<&mut QWebEngineUrlRequestJob>);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[base = QWebEngineUrlSchemeHandler]
        /// The color the served documents are painted, so a page never
        /// flashes white before its QML is drawn over it.
        #[qproperty(QString, background)]
        #[namespace = "ion"]
        type Pages = super::PagesRust;
    }

    unsafe extern "RustQt" {
        /// Serve `ion:` on `profile` (a QML `WebEngineProfile`).
        #[qinvokable]
        unsafe fn attach(self: Pin<&mut Pages>, profile: *mut QObject) -> bool;

        #[cxx_override]
        #[cxx_name = "requestStarted"]
        unsafe fn request_started(self: Pin<&mut Pages>, job: *mut QWebEngineUrlRequestJob);
    }

    extern "RustQt" {
        /// The id of the built-in page `url` shows ("settings"), or "".
        #[qinvokable]
        #[cxx_name = "pageOf"]
        fn page_of(self: &Pages, url: &QUrl) -> QString;

        /// The part of `url` after the page ("privacy" in
        /// `ion://settings/privacy`).
        #[qinvokable]
        #[cxx_name = "pathOf"]
        fn path_of(self: &Pages, url: &QUrl) -> QString;

        /// `ion://<id>` or `ion://<id>/<path>`.
        #[qinvokable]
        fn url(self: &Pages, id: &QString, path: &QString) -> QString;
    }
}

use std::pin::Pin;

use cxx_qt::casting::Upcast;
use cxx_qt_lib::{QString, QUrl};

#[derive(Default)]
pub struct PagesRust {
    background: QString,
}

/// Declare the `ion:` scheme. Must run before the application object exists.
pub fn register_scheme() {
    qobject::register_ion_scheme();
}

fn parse(url: &QUrl) -> Option<(ion_core::pages::Page, String)> {
    let url = url.to_string();
    ion_core::pages::parse(&url).map(|(page, path)| (page, path.to_owned()))
}

impl qobject::Pages {
    unsafe fn attach(self: Pin<&mut Self>, profile: *mut qobject::QObject) -> bool {
        let handler: Pin<&mut qobject::QWebEngineUrlSchemeHandler> = self.upcast_pin();
        // SAFETY: both are live QObjects; the profile doesn't own the
        // handler, which as a QML singleton outlives it.
        unsafe { qobject::install_ion_scheme_handler(profile, handler.get_unchecked_mut()) }
    }

    unsafe fn request_started(self: Pin<&mut Self>, job: *mut qobject::QWebEngineUrlRequestJob) {
        // SAFETY: QtWebEngine hands over a live job for the length of the call.
        let Some(job) = (unsafe { job.as_mut() }) else {
            return;
        };
        // SAFETY: the job is not moved out of.
        let mut job = unsafe { Pin::new_unchecked(job) };
        let url = qobject::job_url(&job).to_string();
        match ion_core::pages::document(&url, &self.background.to_string()) {
            Some(html) => qobject::reply_html(job.as_mut(), &QString::from(html.as_str())),
            None => qobject::fail_job_not_found(job.as_mut()),
        }
    }

    fn page_of(&self, url: &QUrl) -> QString {
        QString::from(parse(url).map_or("", |(page, _)| page.id))
    }

    fn path_of(&self, url: &QUrl) -> QString {
        QString::from(
            parse(url)
                .map(|(_, path)| path)
                .unwrap_or_default()
                .as_str(),
        )
    }

    fn url(&self, id: &QString, path: &QString) -> QString {
        QString::from(ion_core::pages::url(&id.to_string(), &path.to_string()).as_str())
    }
}
