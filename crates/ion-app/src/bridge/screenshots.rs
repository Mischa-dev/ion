//! `Screenshots` QML singleton: where page screenshots are saved
//! (`ion_platform::screenshots`) and copying them to the clipboard.
//!
//! QML grabs the web view with `grabToImage`, saves the result to
//! `savePath(...)`, then calls `copyToClipboard` with the same path.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/screenshots.h");

        #[cxx_name = "copyImageFileToClipboard"]
        fn copy_image_file_to_clipboard(path: &QString) -> bool;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[namespace = "ion"]
        type Screenshots = super::ScreenshotsRust;

        /// A path for a new screenshot of a page titled `title`, taken at
        /// `stamp` (local time, e.g. "2026-10-07 11-42-03"). Creates the
        /// folder; "" if it can't be.
        #[qinvokable]
        #[cxx_name = "savePath"]
        fn save_path(self: &Screenshots, title: &QString, stamp: &QString) -> QString;

        /// Put the image saved at `path` on the clipboard.
        #[qinvokable]
        #[cxx_name = "copyToClipboard"]
        fn copy_to_clipboard(self: &Screenshots, path: &QString) -> bool;
    }
}

use std::path::PathBuf;

use cxx_qt_lib::QString;
use ion_platform::screenshots;

#[derive(Default)]
pub struct ScreenshotsRust;

impl qobject::Screenshots {
    fn save_path(&self, title: &QString, stamp: &QString) -> QString {
        let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
            return QString::default();
        };
        let folder = screenshots::folder(&home);
        if let Err(err) = std::fs::create_dir_all(&folder) {
            eprintln!("ion: could not create {}: {err}", folder.display());
            return QString::default();
        }
        let path = folder.join(screenshots::file_name(
            &title.to_string(),
            &stamp.to_string(),
        ));
        QString::from(path.to_string_lossy().as_ref())
    }

    fn copy_to_clipboard(&self, path: &QString) -> bool {
        qobject::copy_image_file_to_clipboard(path)
    }
}
