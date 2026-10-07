//! `ThemeEngine` QML element: resolves the active palette with `ion_theme` and
//! exposes it as color properties. `qml/Theme.qml` owns the one instance and
//! re-exports its colors as design tokens.
//!
//! It re-resolves when its inputs change (source, theme name, palette path,
//! system light/dark and accent) and when the watched palette file changes.
//!
//! It also decides what web pages see of the theme (`theme.pages`): it sets
//! the `prefers-color-scheme` QtWebEngine hands to pages, and tells each
//! `BrowserTab` whether to force-darken its page (`darkenPage`) and what CSS
//! to add to it (`siteCssScript`), from `[theme.sites]` in the config.

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
        #[qproperty(QString, pages)]
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
        // Whether pages without a dark style should be force-darkened, before
        // per-site settings (see `darkenPage`).
        #[qproperty(bool, darken_pages, cxx_name = "darkenPages", READ, NOTIFY = page_scheme_changed)]
        // A user script adding `[theme.sites]` CSS to pages; empty when none.
        #[qproperty(QString, site_css_script, cxx_name = "siteCssScript", READ, NOTIFY = page_scheme_changed)]
        type ThemeEngine = super::ThemeEngineRust;

        #[qsignal]
        #[cxx_name = "paletteChanged"]
        fn palette_changed(self: Pin<&mut ThemeEngine>);

        /// What pages see of the theme changed; open pages must re-apply their
        /// settings to pick it up.
        #[qsignal]
        #[cxx_name = "pageSchemeChanged"]
        fn page_scheme_changed(self: Pin<&mut ThemeEngine>);

        /// Hand pages the theme's light/dark again. QtWebEngine resets it when
        /// a tab is created, so each tab calls this once its page exists.
        #[qinvokable]
        #[cxx_name = "applyPageScheme"]
        fn apply_page_scheme(self: &ThemeEngine);

        /// Whether the page at `url` should be force-darkened: `darkenPages`
        /// unless a `[theme.sites]` entry says otherwise.
        #[qinvokable]
        #[cxx_name = "darkenPage"]
        fn darken_page(self: &ThemeEngine, url: &QString) -> bool;

        /// Resolve the palette again. Runs on its own when an input changes.
        #[qinvokable]
        fn reload(self: Pin<&mut ThemeEngine>);
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/theme.h");

        #[cxx_name = "setWebColorScheme"]
        fn set_web_color_scheme(scheme: i32);
    }

    impl cxx_qt::Initialize for ThemeEngine {}
    impl cxx_qt::Threading for ThemeEngine {}
}

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QColor, QString, QStringList};
use ion_theme::{
    Color, Dirs, FileWatcher, PageTheming, Palette, Scheme, SiteTheme, SystemAppearance,
    ThemeSource,
};

pub struct ThemeEngineRust {
    source: QString,
    theme_name: QString,
    palette_path: QString,
    system_dark: bool,
    system_accent: QColor,
    pages: QString,

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
    darken_pages: bool,
    /// The scheme pages see, as passed to `set_web_color_scheme`.
    page_scheme: i32,
    /// The system's light/dark when pages follow it, so they re-apply when
    /// it changes.
    page_system_dark: Option<bool>,
    /// The palette's darkness pages were last told about: per-site
    /// `darken = true` depends on it whatever `theme.pages` is.
    page_dark: Option<bool>,
    site_css_script: QString,
    /// `[theme.sites]`, with site names lower-cased.
    sites: BTreeMap<String, SiteTheme>,
    config_subscription: Option<ion_config::Subscription>,

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
            pages: QString::default(),
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
            darken_pages: false,
            // Not a valid scheme, so the first reload always sets it.
            page_scheme: -1,
            page_system_dark: None,
            page_dark: None,
            site_css_script: QString::default(),
            sites: BTreeMap::new(),
            config_subscription: None,
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

    /// Decide what pages see of the current palette and hand QtWebEngine the
    /// scheme. Returns whether anything changed.
    fn update_pages(&mut self, pages: PageTheming) -> bool {
        let scheme = if self.dark {
            Scheme::Dark
        } else {
            Scheme::Light
        };
        let page_scheme = match pages.scheme(scheme) {
            None => 0,
            Some(Scheme::Light) => 1,
            Some(Scheme::Dark) => 2,
        };
        let darken = pages.darken(scheme);
        let system_dark = (page_scheme == 0).then_some(self.system_dark);
        if page_scheme != self.page_scheme {
            qobject::set_web_color_scheme(page_scheme);
        }
        let state = (page_scheme, darken, system_dark, Some(self.dark));
        let old = (
            self.page_scheme,
            self.darken_pages,
            self.page_system_dark,
            self.page_dark,
        );
        (
            self.page_scheme,
            self.darken_pages,
            self.page_system_dark,
            self.page_dark,
        ) = state;
        state != old
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
        self.as_mut()
            .on_pages_changed(|this| this.reload())
            .release();
        let qt_thread = self.qt_thread();
        let subscription = ion_config::global().subscribe(move |_| {
            let _ = qt_thread.queue(|this| this.reload_sites());
        });
        self.as_mut().rust_mut().config_subscription = Some(subscription);
        self.as_mut().reload_sites();
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

        let pages = self.pages().to_string().parse::<PageTheming>();
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
        let error = error.or(pages.as_ref().err().cloned());
        let page_changed = rust.update_pages(pages.unwrap_or_default());
        let error = QString::from(error.unwrap_or_default().as_str());
        let error_changed = rust.error != error;
        rust.error = error;
        rust.theme_ids = dirs
            .theme_ids()
            .iter()
            .map(|id| QString::from(id.as_str()))
            .collect();
        self.as_mut().palette_changed();
        if page_changed {
            self.as_mut().page_scheme_changed();
        }
        if error_changed {
            self.error_changed();
        }
    }

    fn apply_page_scheme(&self) {
        qobject::set_web_color_scheme(self.page_scheme);
    }

    fn darken_page(&self, url: &QString) -> bool {
        ion_theme::sites::darken(&self.sites, &url.to_string(), self.dark, self.darken_pages)
    }

    /// Re-read `[theme.sites]`; tells pages when it changed.
    fn reload_sites(mut self: Pin<&mut Self>) {
        let sites: BTreeMap<String, SiteTheme> = ion_config::global()
            .config()
            .theme
            .sites
            .iter()
            .map(|(site, theme)| {
                let site = site.trim_end_matches('.').to_ascii_lowercase();
                let theme = SiteTheme {
                    darken: theme.darken,
                    css: theme.css.clone(),
                };
                (site, theme)
            })
            .collect();
        if sites == self.sites {
            return;
        }
        let script = QString::from(ion_theme::sites::css_script(&sites).as_str());
        let mut rust = self.as_mut().rust_mut();
        rust.sites = sites;
        rust.site_css_script = script;
        self.page_scheme_changed();
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
