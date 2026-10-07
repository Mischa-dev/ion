//! `Reader` QML singleton: reader mode, from `ion_reader`.
//!
//! QML hands over a page's HTML and URL plus the theme colors, and gets back
//! a standalone reader page to load in the tab (or "" when the page has no
//! article).

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
        #[namespace = "ion"]
        type Reader = super::ReaderRust;

        /// The reader page for `html` loaded from `url`, or "" if there is no
        /// article in it. `style` is a JSON object with `background`, `text`,
        /// `muted`, `accent`, `surface` and `border` colors (`#rrggbb`) and
        /// optionally `fontSize` (px) and `serif` (bool).
        #[qinvokable]
        fn render(self: &Reader, html: &QString, url: &QString, style: &QString) -> QString;
    }
}

use cxx_qt_lib::QString;
use ion_reader::Style;

#[derive(Default)]
pub struct ReaderRust;

fn style_from_json(json: &str) -> Style {
    let value: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    let mut style = Style::default();
    let color = |key: &str, slot: &mut String| {
        if let Some(c) = value[key].as_str() {
            *slot = c.to_owned();
        }
    };
    color("background", &mut style.background);
    color("text", &mut style.text);
    color("muted", &mut style.muted);
    color("accent", &mut style.accent);
    color("surface", &mut style.surface);
    color("border", &mut style.border);
    if let Some(size) = value["fontSize"].as_u64() {
        style.font_size = u32::try_from(size).unwrap_or(style.font_size);
    }
    if let Some(serif) = value["serif"].as_bool() {
        style.serif = serif;
    }
    style
}

impl qobject::Reader {
    fn render(&self, html: &QString, url: &QString, style: &QString) -> QString {
        match ion_reader::extract(&html.to_string(), &url.to_string()) {
            Some(article) => {
                let page = ion_reader::render(&article, &style_from_json(&style.to_string()));
                QString::from(page.as_str())
            }
            None => QString::default(),
        }
    }
}
