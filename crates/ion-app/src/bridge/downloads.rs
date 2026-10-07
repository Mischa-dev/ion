//! `Downloads` QML singleton: download folders, file names and progress text
//! from `ion_basics::downloads`. The panel itself is `DownloadsPanel.qml`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[namespace = "ion"]
        type Downloads = super::DownloadsRust;

        /// Folder a download should be saved in: the per-type folder if one is
        /// set for its kind, otherwise `default_dir`.
        #[qinvokable]
        #[cxx_name = "targetDirectory"]
        fn target_directory(
            self: &Downloads,
            file_name: &QString,
            mime_type: &QString,
            default_dir: &QString,
        ) -> QString;

        /// A sanitized file name that does not exist in `dir` yet.
        #[qinvokable]
        #[cxx_name = "uniqueFileName"]
        fn unique_file_name(self: &Downloads, dir: &QString, name: &QString) -> QString;

        /// `file://` URL of `name` in `dir`, or of `dir` itself when `name` is
        /// empty, for opening with `Qt.openUrlExternally`.
        #[qinvokable]
        #[cxx_name = "fileUrl"]
        fn file_url(self: &Downloads, dir: &QString, name: &QString) -> QUrl;

        /// Send downloads of `category` ("document", "image", "audio",
        /// "video", "archive", "other") to `folder`; empty clears it.
        /// Returns false for an unknown category.
        #[qinvokable]
        #[cxx_name = "setFolder"]
        fn set_folder(self: Pin<&mut Downloads>, category: &QString, folder: &QString) -> bool;

        /// Free a download request the panel no longer shows. The profile
        /// keeps every request until it is deleted.
        #[qinvokable]
        fn release(self: &Downloads, request: &QVariant);

        /// The status line under a download's file name.
        #[qinvokable]
        #[cxx_name = "statusText"]
        fn status_text(
            self: &Downloads,
            state: i32,
            paused: bool,
            received: i64,
            total: i64,
            bytes_per_second: f64,
            interrupt_reason: &QString,
        ) -> QString;

        /// Fraction done in 0..1, or -1 when the size is unknown.
        #[qinvokable]
        fn fraction(self: &Downloads, state: i32, received: i64, total: i64) -> f64;

        /// Smooth a bytes-per-second sample against the previous estimate.
        #[qinvokable]
        #[cxx_name = "smoothRate"]
        fn smooth_rate(self: &Downloads, previous: f64, sample: f64) -> f64;
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/downloads.h");

        #[cxx_name = "deleteLaterObject"]
        fn delete_later_object(value: &QVariant);
    }
}

use std::path::Path;
use std::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QUrl, QVariant};
use ion_basics::downloads::{self, Category, FolderRules, Progress, State};

#[derive(Default)]
pub struct DownloadsRust {
    rules: FolderRules,
}

fn bytes(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

fn progress(state: i32, received: i64, total: i64) -> Progress {
    Progress {
        state: State::from_qt(state),
        paused: false,
        received: bytes(received),
        total: (total > 0).then(|| bytes(total)),
        bytes_per_second: 0.0,
        interrupt_reason: String::new(),
    }
}

impl qobject::Downloads {
    fn target_directory(
        &self,
        file_name: &QString,
        mime_type: &QString,
        default_dir: &QString,
    ) -> QString {
        let default_dir = default_dir.to_string();
        QString::from(self.rules.folder_for(
            &file_name.to_string(),
            &mime_type.to_string(),
            &default_dir,
        ))
    }

    fn unique_file_name(&self, dir: &QString, name: &QString) -> QString {
        let dir = dir.to_string();
        QString::from(downloads::unique_file_name(Path::new(&dir), &name.to_string()).as_str())
    }

    fn file_url(&self, dir: &QString, name: &QString) -> QUrl {
        let dir = dir.to_string();
        let name = name.to_string();
        let path = if name.is_empty() {
            Path::new(&dir).to_path_buf()
        } else {
            Path::new(&dir).join(name)
        };
        QUrl::from_local_file(&QString::from(path.to_string_lossy().as_ref()))
    }

    fn set_folder(self: Pin<&mut Self>, category: &QString, folder: &QString) -> bool {
        let Some(category) = Category::from_name(&category.to_string()) else {
            return false;
        };
        self.rust_mut()
            .get_mut()
            .rules
            .set(category, &folder.to_string());
        true
    }

    fn status_text(
        &self,
        state: i32,
        paused: bool,
        received: i64,
        total: i64,
        bytes_per_second: f64,
        interrupt_reason: &QString,
    ) -> QString {
        let progress = Progress {
            paused,
            bytes_per_second,
            interrupt_reason: interrupt_reason.to_string(),
            ..progress(state, received, total)
        };
        QString::from(progress.status_text().as_str())
    }

    fn fraction(&self, state: i32, received: i64, total: i64) -> f64 {
        progress(state, received, total).fraction().unwrap_or(-1.0)
    }

    fn smooth_rate(&self, previous: f64, sample: f64) -> f64 {
        downloads::smooth_rate(previous, sample)
    }

    fn release(&self, request: &QVariant) {
        qobject::delete_later_object(request);
    }
}
