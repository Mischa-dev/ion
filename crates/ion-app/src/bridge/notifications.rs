//! `DesktopNotifications` QML singleton: web notifications as desktop
//! notifications (org.freedesktop.Notifications on Linux, C++ in
//! `cpp/notifications.cpp`). `NotificationToasts.qml` uses them when
//! `available`, and its own toasts otherwise.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/notifications.h");

        #[cxx_name = "desktopNotificationsAvailable"]
        fn desktop_notifications_available() -> bool;

        #[cxx_name = "showDesktopNotification"]
        fn show_desktop_notification(
            title: &QString,
            body: &QString,
            source: &QString,
            icon: &QVariant,
            replaces_id: u32,
        ) -> u32;

        #[cxx_name = "closeDesktopNotification"]
        fn close_desktop_notification(id: u32);
    }

    #[namespace = "ion"]
    extern "Rust" {
        /// Called by the C++ watcher when a notification is clicked.
        #[cxx_name = "desktopNotificationClicked"]
        fn desktop_notification_clicked(id: u32, activation_token: &QString);

        /// Called by the C++ watcher when a notification goes away.
        #[cxx_name = "desktopNotificationClosed"]
        fn desktop_notification_closed(id: u32);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(bool, available, READ, CONSTANT)]
        #[namespace = "ion"]
        type DesktopNotifications = super::DesktopNotificationsRust;

        /// Show a notification; returns its id, or 0 if it couldn't be
        /// shown. `source` names the site; `icon` is the page's image, if
        /// any; `replaces` is an earlier id to update in place, or 0.
        #[qinvokable]
        fn show(
            self: &DesktopNotifications,
            title: &QString,
            body: &QString,
            source: &QString,
            icon: &QVariant,
            replaces: u32,
        ) -> u32;

        /// Take notification `id` down.
        #[qinvokable]
        fn close(self: &DesktopNotifications, id: u32);

        /// Notification `id` was clicked; `activation_token` lets the
        /// window come forward on Wayland (`Platform.prepareActivation`).
        #[qsignal]
        fn clicked(self: Pin<&mut DesktopNotifications>, id: u32, activation_token: &QString);

        /// Notification `id` went away (dismissed, expired or closed).
        #[qsignal]
        fn closed(self: Pin<&mut DesktopNotifications>, id: u32);
    }

    impl cxx_qt::Threading for DesktopNotifications {}
    impl cxx_qt::Initialize for DesktopNotifications {}
}

use std::pin::Pin;
use std::sync::Mutex;

use cxx_qt::{CxxQtThread, Threading};
use cxx_qt_lib::{QString, QVariant};

pub struct DesktopNotificationsRust {
    available: bool,
}

impl Default for DesktopNotificationsRust {
    fn default() -> Self {
        Self {
            available: qobject::desktop_notifications_available(),
        }
    }
}

static THREAD: Mutex<Option<CxxQtThread<qobject::DesktopNotifications>>> = Mutex::new(None);

fn with_thread(f: impl FnOnce(Pin<&mut qobject::DesktopNotifications>) + Send + 'static) {
    if let Some(thread) = THREAD.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = thread.queue(f);
    }
}

fn desktop_notification_clicked(id: u32, activation_token: &QString) {
    let token = activation_token.to_string();
    with_thread(move |n| n.clicked(id, &QString::from(token.as_str())));
}

fn desktop_notification_closed(id: u32) {
    with_thread(move |n| n.closed(id));
}

impl cxx_qt::Initialize for qobject::DesktopNotifications {
    fn initialize(self: Pin<&mut Self>) {
        *THREAD.lock().unwrap_or_else(|e| e.into_inner()) = Some(self.qt_thread());
    }
}

impl qobject::DesktopNotifications {
    fn show(
        &self,
        title: &QString,
        body: &QString,
        source: &QString,
        icon: &QVariant,
        replaces: u32,
    ) -> u32 {
        if !self.available {
            return 0;
        }
        qobject::show_desktop_notification(title, body, source, icon, replaces)
    }

    fn close(&self, id: u32) {
        if self.available {
            qobject::close_desktop_notification(id);
        }
    }
}
