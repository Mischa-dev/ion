//! `Platform` QML singleton: desktop integration.
//!
//! [`startup`] runs first thing in `main`: it sets QtWebEngine's Chromium
//! switches and, if Ion is already running, hands this launch's URLs to it.
//! The singleton then delivers URLs from later launches and from macOS
//! open-URL events to QML through `openRequested`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/platform.h");

        #[cxx_name = "installPlatformIntegration"]
        fn install_platform_integration();

        #[cxx_name = "setActivationToken"]
        fn set_activation_token(token: &QString);
    }

    #[namespace = "ion"]
    extern "Rust" {
        /// Called by the C++ event filter for macOS open-URL events.
        #[cxx_name = "openUrlFromSystem"]
        fn open_url_from_system(url: &QString);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[namespace = "ion"]
        type Platform = super::PlatformRust;

        /// Open `urls` as tabs and bring the window forward. `urls` may be
        /// empty (Ion launched again without arguments): just come forward.
        #[qsignal]
        #[cxx_name = "openRequested"]
        fn open_requested(self: Pin<&mut Platform>, urls: &QStringList, activation_token: &QString);

        /// Call right before `requestActivate()` with the token from
        /// `openRequested`, so Wayland compositors let the window take focus.
        #[qinvokable]
        #[cxx_name = "prepareActivation"]
        fn prepare_activation(self: &Platform, activation_token: &QString);
    }

    impl cxx_qt::Threading for Platform {}
    impl cxx_qt::Initialize for Platform {}
}

use std::pin::Pin;
use std::sync::Mutex;

use cxx_qt::{CxxQtThread, Threading};
use cxx_qt_lib::{QString, QStringList};
use ion_platform::chromium;
use ion_platform::instance::{self, Claim, Listener, Request};

/// The browser profile; one running Ion per profile.
const PROFILE: &str = "Default";

#[derive(Default)]
pub struct PlatformRust;

/// Requests wait here until the QML singleton exists.
enum Sink {
    Pending(Vec<Request>),
    Ready(CxxQtThread<qobject::Platform>),
}

static SINK: Mutex<Sink> = Mutex::new(Sink::Pending(Vec::new()));
static LISTENER: Mutex<Option<Listener>> = Mutex::new(None);

/// Run before anything else in `main`. Returns `false` when another Ion took
/// over this launch and the process should exit.
pub fn startup() -> bool {
    let user_flags = std::env::var(chromium::FLAGS_VAR).ok();
    if let Some(flags) =
        chromium::merged_flags(&chromium::Options::from_env(), user_flags.as_deref())
    {
        // SAFETY: called at the top of `main`, before any other thread exists.
        unsafe { std::env::set_var(chromium::FLAGS_VAR, flags) };
    }

    let path = instance::socket_path(ion_core::APP_ID, PROFILE);
    match instance::claim(&path, &Request::from_launch(std::env::args().skip(1))) {
        Claim::Forwarded => false,
        Claim::Primary(listener) => {
            *LISTENER.lock().unwrap_or_else(|e| e.into_inner()) = listener;
            true
        }
    }
}

fn deliver(request: Request) {
    let mut sink = SINK.lock().unwrap_or_else(|e| e.into_inner());
    match &mut *sink {
        Sink::Pending(queue) => queue.push(request),
        Sink::Ready(thread) => {
            let _ = thread.queue(move |platform| emit(platform, request));
        }
    }
}

fn emit(platform: Pin<&mut qobject::Platform>, request: Request) {
    let mut urls = QStringList::default();
    for url in &request.urls {
        urls.append(QString::from(url.as_str()));
    }
    let token = QString::from(request.activation_token.as_deref().unwrap_or_default());
    platform.open_requested(&urls, &token);
}

fn open_url_from_system(url: &QString) {
    deliver(Request {
        urls: vec![url.to_string()],
        activation_token: None,
    });
}

impl cxx_qt::Initialize for qobject::Platform {
    fn initialize(self: Pin<&mut Self>) {
        qobject::install_platform_integration();

        let thread = self.qt_thread();
        let pending = {
            let mut sink = SINK.lock().unwrap_or_else(|e| e.into_inner());
            match std::mem::replace(&mut *sink, Sink::Ready(thread.clone())) {
                Sink::Pending(queue) => queue,
                Sink::Ready(_) => Vec::new(),
            }
        };
        for request in pending {
            let _ = thread.queue(move |platform| emit(platform, request));
        }

        let listener = LISTENER.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(listener) = listener {
            let spawned = std::thread::Builder::new()
                .name("ion-instance".into())
                .spawn(move || listener.serve(deliver));
            if let Err(e) = spawned {
                eprintln!("ion: cannot listen for other launches: {e}");
            }
        }
    }
}

impl qobject::Platform {
    fn prepare_activation(&self, activation_token: &QString) {
        qobject::set_activation_token(activation_token);
    }
}
