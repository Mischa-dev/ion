//! `DownloadHistory` QML singleton: past downloads for the Downloads page,
//! backed by `ion_basics::download_log` and saved on every change.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(i32, revision, READ, NOTIFY)]
        #[namespace = "ion"]
        type DownloadHistory = super::DownloadHistoryRust;

        /// Remember a download that ended. `outcome` is "completed",
        /// "cancelled" or "failed".
        #[qinvokable]
        fn record(
            self: Pin<&mut DownloadHistory>,
            url: &QString,
            directory: &QString,
            file_name: &QString,
            mime_type: &QString,
            bytes: i64,
            outcome: &QString,
        );

        /// Past downloads matching `query`, newest first, as a JSON array of
        /// `{id, url, host, directory, fileName, size, outcome, finished,
        /// category, exists}`.
        #[qinvokable]
        fn list(self: &DownloadHistory, query: &QString) -> QString;

        /// Forget one download; the file stays.
        #[qinvokable]
        fn remove(self: Pin<&mut DownloadHistory>, id: i64);

        /// Forget every past download; the files stay.
        #[qinvokable]
        fn clear(self: Pin<&mut DownloadHistory>);
    }
}

use core::pin::Pin;
use std::path::PathBuf;

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use ion_basics::download_log::{DownloadLog, Outcome, Record};
use ion_basics::downloads::{Category, format_bytes};

pub struct DownloadHistoryRust {
    log: DownloadLog,
    path: Option<PathBuf>,
    revision: i32,
}

impl Default for DownloadHistoryRust {
    fn default() -> Self {
        let path = ion_session::paths::data_dir().map(|d| d.join("downloads.json"));
        let log = path
            .as_deref()
            .map(|p| {
                DownloadLog::load(p).unwrap_or_else(|err| {
                    eprintln!("ion: could not read the downloads list: {err}");
                    DownloadLog::default()
                })
            })
            .unwrap_or_default();
        Self {
            log,
            path,
            revision: 0,
        }
    }
}

impl qobject::DownloadHistory {
    fn changed(mut self: Pin<&mut Self>) {
        if let Some(path) = &self.path
            && let Err(err) = self.log.save(path)
        {
            eprintln!("ion: could not save the downloads list: {err}");
        }
        let revision = self.revision.wrapping_add(1);
        self.as_mut().rust_mut().revision = revision;
        self.revision_changed();
    }

    fn record(
        mut self: Pin<&mut Self>,
        url: &QString,
        directory: &QString,
        file_name: &QString,
        mime_type: &QString,
        bytes: i64,
        outcome: &QString,
    ) {
        let record = Record {
            id: 0,
            url: url.to_string(),
            directory: directory.to_string(),
            file_name: file_name.to_string(),
            mime_type: mime_type.to_string(),
            bytes: u64::try_from(bytes).unwrap_or(0),
            outcome: Outcome::from_name(&outcome.to_string()).unwrap_or(Outcome::Failed),
            finished: ion_session::history::now(),
        };
        self.as_mut().rust_mut().log.record(record);
        self.changed();
    }

    fn list(&self, query: &QString) -> QString {
        let items: Vec<serde_json::Value> = self
            .log
            .search(&query.to_string())
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "url": r.url,
                    "host": ion_basics::display_host(&r.url).unwrap_or_default(),
                    "directory": r.directory,
                    "fileName": r.file_name,
                    "size": if r.bytes > 0 { format_bytes(r.bytes) } else { String::new() },
                    "outcome": r.outcome.as_str(),
                    "finished": r.finished,
                    "category": Category::classify(&r.file_name, &r.mime_type).name(),
                    "exists": r.path().exists(),
                })
            })
            .collect();
        QString::from(serde_json::Value::Array(items).to_string().as_str())
    }

    fn remove(mut self: Pin<&mut Self>, id: i64) {
        let Ok(id) = u64::try_from(id) else { return };
        if self.as_mut().rust_mut().log.remove(id) {
            self.changed();
        }
    }

    fn clear(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().log.clear();
        self.changed();
    }
}
