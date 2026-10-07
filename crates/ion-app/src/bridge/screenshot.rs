//! `Screenshot` QML singleton: full-page tile planning and file names from
//! `ion_basics::screenshot`, plus saving, copying and stitching the images
//! QML grabs (C++ in `cpp/screenshot.cpp`). The capture flow itself is
//! `ScreenshotTool.qml`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qlist.h");
        type QList_f64 = cxx_qt_lib::QList<f64>;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[namespace = "ion"]
        type Screenshot = super::ScreenshotRust;

        /// Scroll positions (CSS px) that cover a page of `total` height
        /// through a `viewport`-high view; the last one is bottom-aligned.
        #[qinvokable]
        #[cxx_name = "tileOffsets"]
        fn tile_offsets(self: &Screenshot, total: f64, viewport: f64) -> QList_f64;

        /// Height (CSS px) of a full-page capture, after the size limit.
        #[qinvokable]
        #[cxx_name = "capturedHeight"]
        fn captured_height(self: &Screenshot, total: f64, viewport: f64) -> f64;

        /// "Screenshot example.com 2026-10-07 11.05.03.png".
        #[qinvokable]
        #[cxx_name = "fileName"]
        fn file_name(self: &Screenshot, url: &QUrl, stamp: &QString) -> QString;

        /// Save a grabbed image as PNG at `path`.
        #[qinvokable]
        fn save(self: &Screenshot, image: &QVariant, path: &QString) -> bool;

        /// Put a grabbed image on the clipboard.
        #[qinvokable]
        fn copy(self: &Screenshot, image: &QVariant);

        /// Paint a tile grabbed at scroll position `y` onto `canvas` (empty
        /// for the first tile) and return the page-high image.
        #[qinvokable]
        fn paste(
            self: &Screenshot,
            canvas: &QVariant,
            tile: &QVariant,
            y: f64,
            viewport: f64,
            total: f64,
        ) -> QVariant;
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/screenshot.h");

        #[cxx_name = "saveImage"]
        fn save_image(image: &QVariant, path: &QString) -> bool;

        #[cxx_name = "copyImage"]
        fn copy_image(image: &QVariant);

        #[cxx_name = "pasteTile"]
        fn paste_tile(
            canvas: &QVariant,
            tile: &QVariant,
            y: f64,
            viewport: f64,
            total: f64,
        ) -> QVariant;
    }
}

use cxx_qt_lib::{QList, QString, QUrl, QVariant};
use ion_basics::screenshot;

#[derive(Default)]
pub struct ScreenshotRust;

impl qobject::Screenshot {
    fn tile_offsets(&self, total: f64, viewport: f64) -> QList<f64> {
        let mut list = QList::default();
        for offset in screenshot::tile_offsets(total, viewport) {
            list.append(offset);
        }
        list
    }

    fn captured_height(&self, total: f64, viewport: f64) -> f64 {
        screenshot::captured_height(total, viewport)
    }

    fn file_name(&self, url: &QUrl, stamp: &QString) -> QString {
        QString::from(screenshot::file_name(&url.to_string(), &stamp.to_string()).as_str())
    }

    fn save(&self, image: &QVariant, path: &QString) -> bool {
        qobject::save_image(image, path)
    }

    fn copy(&self, image: &QVariant) {
        qobject::copy_image(image);
    }

    fn paste(
        &self,
        canvas: &QVariant,
        tile: &QVariant,
        y: f64,
        viewport: f64,
        total: f64,
    ) -> QVariant {
        qobject::paste_tile(canvas, tile, y, viewport, total)
    }
}
