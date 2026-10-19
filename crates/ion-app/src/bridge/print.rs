//! `Printing` QML singleton: page ranges, paper and file names from
//! `ion_print`, and sending the engine's PDF of a page to a printer (C++ in
//! `cpp/print.cpp`). The preview itself is `PrintPreview.qml`.

// `print` takes one argument per print setting; QML can't pass a struct.
#![allow(clippy::too_many_arguments)]

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qlist.h");
        type QList_i32 = cxx_qt_lib::QList<i32>;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(bool, busy, READ, NOTIFY)]
        #[namespace = "ion"]
        type Printing = super::PrintingRust;

        /// The printers the system offers.
        #[qinvokable]
        fn printers(self: &Printing) -> QStringList;

        /// The system's default printer, or "".
        #[qinvokable]
        #[cxx_name = "defaultPrinter"]
        fn default_printer(self: &Printing) -> QString;

        /// Paper sizes as `{ id, label }`; `id` names a
        /// `WebEngineView.PrintedPageSizeId`.
        #[qinvokable]
        fn papers(self: &Printing) -> QVariant;

        /// The paper size usual where Ion runs ("A4" or "Letter").
        #[qinvokable]
        #[cxx_name = "defaultPaper"]
        fn default_paper(self: &Printing) -> QString;

        /// The 0-based pages `text` (like "1-3, 5") picks out of `count`;
        /// empty when it can't be read (see `rangeError`).
        #[qinvokable]
        #[cxx_name = "pageList"]
        fn page_list(self: &Printing, text: &QString, count: i32) -> QList_i32;

        /// Why `text` doesn't pick pages out of `count`, or "".
        #[qinvokable]
        #[cxx_name = "rangeError"]
        fn range_error(self: &Printing, text: &QString, count: i32) -> QString;

        /// "Page title.pdf" for a saved copy of the page.
        #[qinvokable]
        #[cxx_name = "fileName"]
        fn file_name(self: &Printing, title: &QString, url: &QString) -> QString;

        /// A fresh path in the temporary folder for the next preview.
        #[qinvokable]
        #[cxx_name = "previewPath"]
        fn preview_path(self: &Printing) -> QString;

        /// Copy `from` to `to`. False if it fails.
        #[qinvokable]
        #[cxx_name = "copyFile"]
        fn copy_file(self: &Printing, from: &QString, to: &QString) -> bool;

        /// Delete a preview file made from `previewPath`.
        #[qinvokable]
        #[cxx_name = "removePreview"]
        fn remove_preview(self: &Printing, path: &QString);

        /// Print `pages` of the PDF at `path` on `printer` in the background;
        /// `printed` follows.
        #[qinvokable]
        fn print(
            self: Pin<&mut Printing>,
            path: &QString,
            printer: &QString,
            title: &QString,
            pages: &QList_i32,
            copies: i32,
            color: bool,
            duplex: bool,
        );

        /// A `print` finished: `error` is "" once the printer has the job.
        #[qsignal]
        fn printed(self: Pin<&mut Printing>, error: QString);
    }

    impl cxx_qt::Threading for Printing {}

    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/print.h");

        #[cxx_name = "printerNames"]
        fn printer_names() -> QStringList;

        #[cxx_name = "defaultPrinterName"]
        fn default_printer_name() -> QString;

        #[cxx_name = "printPdf"]
        fn print_pdf(
            pdf_path: &QString,
            printer: &QString,
            title: &QString,
            pages: &QList_i32,
            copies: i32,
            color: bool,
            duplex: bool,
        ) -> QString;
    }
}

use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QStringList, QVariant};

#[derive(Default)]
pub struct PrintingRust {
    busy: bool,
}

/// Previews live here, so nothing else in the temporary folder is ever
/// removed by `removePreview`.
fn preview_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("ion-print-{}", std::process::id()))
}

fn locale() -> String {
    ["LC_ALL", "LC_PAPER", "LANG"]
        .iter()
        .filter_map(|key| std::env::var(key).ok())
        .find(|value| !value.is_empty())
        .unwrap_or_default()
}

fn pages(text: &QString, count: i32) -> Result<Vec<u32>, String> {
    ion_print::parse_ranges(&text.to_string(), u32::try_from(count).unwrap_or(0))
}

impl qobject::Printing {
    fn set_busy_flag(mut self: Pin<&mut Self>, busy: bool) {
        self.as_mut().rust_mut().busy = busy;
        self.busy_changed();
    }

    fn printers(&self) -> QStringList {
        qobject::printer_names()
    }

    fn default_printer(&self) -> QString {
        qobject::default_printer_name()
    }

    fn papers(&self) -> QVariant {
        let mut list = QList::<QVariant>::default();
        for paper in ion_print::PAPERS {
            let mut map = QMap::<QMapPair_QString_QVariant>::default();
            map.insert(
                QString::from("id"),
                QVariant::from(&QString::from(paper.id)),
            );
            map.insert(
                QString::from("label"),
                QVariant::from(&QString::from(paper.label)),
            );
            list.append(QVariant::from(&map));
        }
        QVariant::from(&list)
    }

    fn default_paper(&self) -> QString {
        QString::from(ion_print::default_paper(&locale()))
    }

    fn page_list(&self, text: &QString, count: i32) -> QList<i32> {
        let mut list = QList::default();
        for page in pages(text, count).unwrap_or_default() {
            list.append(i32::try_from(page).unwrap_or(i32::MAX));
        }
        list
    }

    fn range_error(&self, text: &QString, count: i32) -> QString {
        QString::from(pages(text, count).err().unwrap_or_default().as_str())
    }

    fn file_name(&self, title: &QString, url: &QString) -> QString {
        QString::from(ion_print::file_name(&title.to_string(), &url.to_string()).as_str())
    }

    fn preview_path(&self) -> QString {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dir = preview_dir();
        if let Err(err) = std::fs::create_dir_all(&dir) {
            eprintln!("ion: print preview folder {}: {err}", dir.display());
        }
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        QString::from(
            dir.join(format!("preview-{n}.pdf"))
                .to_string_lossy()
                .as_ref(),
        )
    }

    fn copy_file(&self, from: &QString, to: &QString) -> bool {
        std::fs::copy(from.to_string(), to.to_string())
            .map_err(|err| eprintln!("ion: could not save {to}: {err}"))
            .is_ok()
    }

    fn remove_preview(&self, path: &QString) {
        let path = path.to_string();
        if Path::new(&path).parent() == Some(preview_dir().as_path()) {
            let _ = std::fs::remove_file(&path);
        }
    }

    fn print(
        mut self: Pin<&mut Self>,
        path: &QString,
        printer: &QString,
        title: &QString,
        pages: &QList<i32>,
        copies: i32,
        color: bool,
        duplex: bool,
    ) {
        if *self.busy() {
            return;
        }
        self.as_mut().set_busy_flag(true);
        let thread = self.qt_thread();
        let (path, printer, title) = (path.to_string(), printer.to_string(), title.to_string());
        let pages: Vec<i32> = pages.iter().copied().collect();
        std::thread::spawn(move || {
            let mut list = QList::<i32>::default();
            for page in pages {
                list.append(page);
            }
            let error = qobject::print_pdf(
                &QString::from(path.as_str()),
                &QString::from(printer.as_str()),
                &QString::from(title.as_str()),
                &list,
                copies,
                color,
                duplex,
            )
            .to_string();
            let _ = thread.queue(move |mut obj| {
                obj.as_mut().set_busy_flag(false);
                obj.printed(QString::from(error.as_str()));
            });
        });
    }
}

impl Drop for PrintingRust {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(preview_dir());
    }
}
