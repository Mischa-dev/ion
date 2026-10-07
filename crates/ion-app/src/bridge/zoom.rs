//! `Zoom` QML singleton: zoom steps and per-site zoom memory from
//! `ion_basics::zoom`, saved to `zoom.txt` in Ion's data directory.

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
        #[qml_singleton]
        #[namespace = "ion"]
        type Zoom = super::ZoomRust;

        /// The next zoom step above `factor`.
        #[qinvokable]
        #[cxx_name = "stepIn"]
        fn step_in(self: &Zoom, factor: f64) -> f64;

        /// The next zoom step below `factor`.
        #[qinvokable]
        #[cxx_name = "stepOut"]
        fn step_out(self: &Zoom, factor: f64) -> f64;

        /// True if `factor` is 100%.
        #[qinvokable]
        #[cxx_name = "isDefault"]
        fn is_default(self: &Zoom, factor: f64) -> bool;

        /// `factor` as a label, "110%".
        #[qinvokable]
        fn label(self: &Zoom, factor: f64) -> QString;

        /// The zoom remembered for `url`'s site, 1.0 if none.
        #[qinvokable]
        #[cxx_name = "factorFor"]
        fn factor_for(self: Pin<&mut Zoom>, url: &QUrl) -> f64;

        /// Remember `factor` for `url`'s site and save it.
        #[qinvokable]
        fn remember(self: Pin<&mut Zoom>, url: &QUrl, factor: f64);
    }
}

use std::path::PathBuf;
use std::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QUrl};
use ion_basics::zoom::{self, SiteZoom};

#[derive(Default)]
pub struct ZoomRust {
    sites: Option<SiteZoom>,
}

/// `zoom.txt` in Ion's data directory, the same one sessions and history use
/// (`ION_DATA_DIR` wins over the platform default).
fn store_path() -> Option<PathBuf> {
    Some(ion_session::paths::data_dir()?.join("zoom.txt"))
}

impl ZoomRust {
    /// The per-site memory, read from disk on first use.
    fn sites(&mut self) -> &mut SiteZoom {
        self.sites.get_or_insert_with(|| {
            store_path()
                .and_then(|path| std::fs::read_to_string(path).ok())
                .map(|text| SiteZoom::from_text(&text))
                .unwrap_or_default()
        })
    }

    fn save(&self) {
        let (Some(sites), Some(path)) = (&self.sites, store_path()) else {
            return;
        };
        let result = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, sites.to_text()));
        if let Err(err) = result {
            eprintln!(
                "ion: could not save zoom levels to {}: {err}",
                path.display()
            );
        }
    }
}

impl qobject::Zoom {
    fn step_in(&self, factor: f64) -> f64 {
        zoom::step_in(factor)
    }

    fn step_out(&self, factor: f64) -> f64 {
        zoom::step_out(factor)
    }

    fn is_default(&self, factor: f64) -> bool {
        zoom::is_default(factor)
    }

    fn label(&self, factor: f64) -> QString {
        QString::from(format!("{}%", zoom::percent(factor)).as_str())
    }

    fn factor_for(self: Pin<&mut Self>, url: &QUrl) -> f64 {
        let url = url.to_string();
        self.rust_mut().get_mut().sites().factor_for(&url)
    }

    fn remember(self: Pin<&mut Self>, url: &QUrl, factor: f64) {
        let url = url.to_string();
        let rust = self.rust_mut().get_mut();
        if rust.sites().remember(&url, factor) {
            rust.save();
        }
    }
}
