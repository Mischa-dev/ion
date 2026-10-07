//! `ThemeEngine` QML element: resolves the active palette with `ion_theme` and
//! exposes it as color properties. `qml/Theme.qml` owns the one instance and
//! re-exports its colors as design tokens.
//!
//! It re-resolves when its inputs change (source, theme name, palette path,
//! system light/dark and accent) and when the watched palette file changes.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qcolor.h");
        type QColor = cxx_qt_lib::QColor;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[namespace = "ion"]
        // Inputs, set from QML / config.
        #[qproperty(QString, source)]
        #[qproperty(QString, theme_name, cxx_name = "themeName")]
        #[qproperty(QString, palette_path, cxx_name = "palettePath")]
        #[qproperty(bool, system_dark, cxx_name = "systemDark")]
        #[qproperty(QColor, system_accent, cxx_name = "systemAccent")]
        // Outputs: the resolved palette.
        #[qproperty(QString, name, READ, NOTIFY = palette_changed)]
        #[qproperty(bool, dark, READ, NOTIFY = palette_changed)]
        #[qproperty(QColor, background, READ, NOTIFY = palette_changed)]
        #[qproperty(QColor, surface, READ, NOTIFY = palette_changed)]
        #[qproperty(QColor, surface_raised, cxx_name = "surfaceRaised", READ, NOTIFY = palette_changed)]
        #[qproperty(QColor, surface_hover, cxx_name = "surfaceHover", READ, NOTIFY = palette_changed)]
        #[qproperty(QColor, border, READ, NOTIFY = palette_changed)]
        #[qproperty(QColor, text, READ, NOTIFY = palette_changed)]
        #[qproperty(QColor, text_muted, cxx_name = "textMuted", READ, NOTIFY = palette_changed)]
        #[qproperty(QColor, accent, READ, NOTIFY = palette_changed)]
        #[qproperty(QColor, on_accent, cxx_name = "onAccent", READ, NOTIFY = palette_changed)]
        #[qproperty(QColor, danger, READ, NOTIFY = palette_changed)]
        #[qproperty(QColor, warning, READ, NOTIFY = palette_changed)]
        #[qproperty(QColor, success, READ, NOTIFY = palette_changed)]
        // themeIds: what a theme picker can offer (built-ins, then installed files).
        #[qproperty(QStringList, theme_ids, cxx_name = "themeIds", READ, NOTIFY = palette_changed)]
        // error: why the configured theme could not be used; empty when it was.
        // Its own signal, so a handler fires once per new error.
        #[qproperty(QString, error, READ, NOTIFY)]
        type ThemeEngine = super::ThemeEngineRust;

        #[qsignal]
        #[cxx_name = "paletteChanged"]
        fn palette_changed(self: Pin<&mut ThemeEngine>);

        /// Resolve the palette again. Runs on its own when an input changes.
        #[qinvokable]
        fn reload(self: Pin<&mut ThemeEngine>);
    }

    impl cxx_qt::Initialize for ThemeEngine {}
    impl cxx_qt::Threading for ThemeEngine {}
}

use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QColor, QString, QStringList};
use ion_theme::{Color, Dirs, FileWatcher, Palette, Scheme, SystemAppearance, ThemeSource};

pub struct ThemeEngineRust {
    source: QString,
    theme_name: QString,
    palette_path: QString,
    system_dark: bool,
    system_accent: QColor,

    name: QString,
    dark: bool,
    background: QColor,
    surface: QColor,
    surface_raised: QColor,
    surface_hover: QColor,
    border: QColor,
    text: QColor,
    text_muted: QColor,
    accent: QColor,
    on_accent: QColor,
    danger: QColor,
    warning: QColor,
    success: QColor,
    theme_ids: QStringList,
    error: QString,

    /// The palette file being watched, and its watcher.
    watched: Option<(PathBuf, FileWatcher)>,
    /// Set while a reload is queued, so a burst of file events reloads once.
    reload_queued: Arc<AtomicBool>,
}

fn qcolor(c: Color) -> QColor {
    QColor::from_rgba(c.r.into(), c.g.into(), c.b.into(), c.a.into())
}

impl Default for ThemeEngineRust {
    fn default() -> Self {
        let mut this = Self {
            source: QString::from("builtin"),
            theme_name: QString::from(ion_theme::source::AUTO),
            palette_path: QString::default(),
            system_dark: true,
            system_accent: QColor::default(),
            name: QString::default(),
            dark: true,
            background: QColor::default(),
            surface: QColor::default(),
            surface_raised: QColor::default(),
            surface_hover: QColor::default(),
            border: QColor::default(),
            text: QColor::default(),
            text_muted: QColor::default(),
            accent: QColor::default(),
            on_accent: QColor::default(),
            danger: QColor::default(),
            warning: QColor::default(),
            success: QColor::default(),
            theme_ids: QStringList::default(),
            error: QString::default(),
            watched: None,
            reload_queued: Arc::default(),
        };
        let fallback = ion_theme::builtin::get(ion_theme::builtin::DEFAULT_DARK)
            .expect("default theme is built in");
        this.apply(&fallback);
        this
    }
}

impl ThemeEngineRust {
    fn apply(&mut self, p: &Palette) {
        self.name = QString::from(p.name.as_str());
        self.dark = p.scheme == Scheme::Dark;
        self.background = qcolor(p.background);
        self.surface = qcolor(p.surface);
        self.surface_raised = qcolor(p.surface_raised);
        self.surface_hover = qcolor(p.surface_hover);
        self.border = qcolor(p.border);
        self.text = qcolor(p.text);
        self.text_muted = qcolor(p.text_muted);
        self.accent = qcolor(p.accent);
        self.on_accent = qcolor(p.on_accent);
        self.danger = qcolor(p.danger);
        self.warning = qcolor(p.warning);
        self.success = qcolor(p.success);
    }

    fn system_appearance(&self) -> SystemAppearance {
        let c = &self.system_accent;
        let channel = |v: i32| v.clamp(0, 255) as u8;
        SystemAppearance {
            dark: self.system_dark,
            accent: c
                .is_valid()
                .then(|| Color::rgb(channel(c.red()), channel(c.green()), channel(c.blue()))),
        }
    }
}

impl cxx_qt::Initialize for qobject::ThemeEngine {
    fn initialize(mut self: Pin<&mut Self>) {
        self.as_mut()
            .on_source_changed(|this| this.reload())
            .release();
        self.as_mut()
            .on_theme_name_changed(|this| this.reload())
            .release();
        self.as_mut()
            .on_palette_path_changed(|this| this.reload())
            .release();
        self.as_mut()
            .on_system_dark_changed(|this| this.reload())
            .release();
        self.as_mut()
            .on_system_accent_changed(|this| this.reload())
            .release();
        self.reload();
    }
}

impl qobject::ThemeEngine {
    fn reload(mut self: Pin<&mut Self>) {
        let dirs = Dirs::from_env();
        let system = self.rust().system_appearance();
        let source = ThemeSource::from_settings(
            &self.source().to_string(),
            &self.theme_name().to_string(),
            &self.palette_path().to_string(),
            &dirs,
        );
        let watch_path = source.as_ref().ok().and_then(|s| s.palette_file(&dirs));
        let watch_error = self.as_mut().watch(watch_path);
        let resolved = source.and_then(|s| s.resolve(&system, &dirs));

        let mut rust = self.as_mut().rust_mut();
        let error = match resolved {
            Ok(palette) => {
                rust.apply(&palette);
                watch_error
            }
            // Keep the current palette: a palette file caught mid-write or
            // mid-edit should not flash the UI back to the default theme.
            Err(e) => Some(e.to_string()),
        };
        let error = QString::from(error.unwrap_or_default().as_str());
        let error_changed = rust.error != error;
        rust.error = error;
        rust.theme_ids = dirs
            .theme_ids()
            .iter()
            .map(|id| QString::from(id.as_str()))
            .collect();
        self.as_mut().palette_changed();
        if error_changed {
            self.error_changed();
        }
    }

    /// Watch `path` for changes (or stop watching). Returns an error message
    /// if the file cannot be watched; the theme still applies, just not live.
    fn watch(mut self: Pin<&mut Self>, path: Option<PathBuf>) -> Option<String> {
        if self.rust().watched.as_ref().map(|(p, _)| p) == path.as_ref() {
            return None;
        }
        let Some(path) = path else {
            self.as_mut().rust_mut().watched = None;
            return None;
        };
        let qt_thread = self.qt_thread();
        let queued = self.rust().reload_queued.clone();
        let watcher = FileWatcher::new(&path, move || {
            if queued.swap(true, Ordering::AcqRel) {
                return;
            }
            let queued = queued.clone();
            // Fails only if the object is gone, and then nothing needs reloading.
            let _ = qt_thread.queue(move |this| {
                queued.store(false, Ordering::Release);
                this.reload();
            });
        });
        match watcher {
            Ok(watcher) => {
                self.as_mut().rust_mut().watched = Some((path, watcher));
                None
            }
            Err(e) => {
                self.as_mut().rust_mut().watched = None;
                Some(format!("cannot watch {}: {e}", path.display()))
            }
        }
    }
}
