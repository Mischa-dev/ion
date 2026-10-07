//! `Config` QML singleton: the live `ion_config::global()` store.
//!
//! Typed properties cover the settings the shell reads today and update by
//! themselves on every reload. Anything else is reachable by dotted key:
//! `Config.revision, Config.value("ui.cornerRadius")` re-evaluates on change.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(i32, revision, READ, NOTIFY)]
        #[qproperty(QString, error, READ, NOTIFY)]
        #[qproperty(QStringList, warnings, READ, NOTIFY)]
        #[qproperty(QString, directory, READ, CONSTANT)]
        #[qproperty(QString, home_page, cxx_name = "homePage", READ, NOTIFY)]
        #[qproperty(
            QString,
            search_engine_name,
            cxx_name = "searchEngineName",
            READ,
            NOTIFY
        )]
        #[qproperty(QString, search_template, cxx_name = "searchTemplate", READ, NOTIFY)]
        #[qproperty(QString, density, READ, NOTIFY)]
        #[qproperty(i32, corner_radius, cxx_name = "cornerRadius", READ, NOTIFY)]
        #[qproperty(QString, tab_layout, cxx_name = "tabLayout", READ, NOTIFY)]
        #[qproperty(bool, animations_enabled, cxx_name = "animationsEnabled", READ, NOTIFY)]
        #[qproperty(f64, animation_speed, cxx_name = "animationSpeed", READ, NOTIFY)]
        #[qproperty(QString, theme_source, cxx_name = "themeSource", READ, NOTIFY)]
        #[qproperty(QString, theme_name, cxx_name = "themeName", READ, NOTIFY)]
        #[namespace = "ion"]
        type Config = super::ConfigRust;

        /// The effective value of a dotted key, or undefined if there is none.
        #[qinvokable]
        fn value(self: &Config, key: &QString) -> QVariant;

        /// Save `value` for `key` in the overrides file. Returns an empty
        /// string on success, else why it was rejected.
        #[qinvokable]
        fn set(self: Pin<&mut Config>, key: &QString, value: &QVariant) -> QString;

        /// Drop the override for `key`. Returns an error message or "".
        #[qinvokable]
        fn reset(self: Pin<&mut Config>, key: &QString) -> QString;

        /// Re-read the files now (they are also watched).
        #[qinvokable]
        fn reload(self: Pin<&mut Config>);

        /// The overrides as a `programs.ion` Nix snippet.
        #[qinvokable]
        #[cxx_name = "overridesAsNix"]
        fn overrides_as_nix(self: &Config) -> QString;
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/config.h");

        #[cxx_name = "plainVariant"]
        fn plain_variant(value: &QVariant) -> QVariant;
    }

    impl cxx_qt::Threading for Config {}
    impl cxx_qt::Initialize for Config {}
}

use std::pin::Pin;
use std::sync::Arc;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QStringList, QVariant};
use ion_config::{State, Subscription, Value};

#[derive(Default)]
pub struct ConfigRust {
    revision: i32,
    error: QString,
    warnings: QStringList,
    directory: QString,
    home_page: QString,
    search_engine_name: QString,
    search_template: QString,
    density: QString,
    corner_radius: i32,
    tab_layout: QString,
    animations_enabled: bool,
    animation_speed: f64,
    theme_source: QString,
    theme_name: QString,
    subscription: Option<Subscription>,
    /// The state last applied, so a queued update for a state that `set`
    /// already applied is a no-op.
    applied: Option<Arc<State>>,
}

impl cxx_qt::Initialize for qobject::Config {
    fn initialize(mut self: Pin<&mut Self>) {
        let store = ion_config::global();
        if let Some(paths) = store.paths() {
            self.as_mut().rust_mut().directory =
                QString::from(paths.dir.to_string_lossy().as_ref());
        }
        self.as_mut().apply(store.state());

        // Reloads happen on the watcher thread; hop to the Qt thread to apply.
        let qt_thread = self.qt_thread();
        // Apply whatever is current when the event runs, never an older state
        // that a `set` on the Qt thread has already replaced.
        let subscription = store.subscribe(move |_| {
            // Fails only once the object is gone, when there is nothing to update.
            let _ = qt_thread.queue(|config| config.apply(ion_config::global().state()));
        });
        self.as_mut().rust_mut().subscription = Some(subscription);
    }
}

impl qobject::Config {
    fn apply(mut self: Pin<&mut Self>, state: Arc<State>) {
        if self
            .applied
            .as_ref()
            .is_some_and(|a| Arc::ptr_eq(a, &state))
        {
            return;
        }
        // Properties are read-only for QML: update the field, then notify.
        macro_rules! update {
            ($field:ident, $changed:ident, $value:expr) => {{
                let value = $value;
                if self.$field != value {
                    self.as_mut().rust_mut().$field = value;
                    self.as_mut().$changed();
                }
            }};
        }
        let text = |s: &str| QString::from(s);
        let config = &state.config;
        let warnings: QStringList = state.warnings.iter().map(|w| text(w)).collect();

        update!(
            home_page,
            home_page_changed,
            text(&config.general.home_page)
        );
        update!(
            search_engine_name,
            search_engine_name_changed,
            text(&config.search.engine)
        );
        update!(
            search_template,
            search_template_changed,
            text(&config.search.template)
        );
        update!(density, density_changed, text(config.ui.density.as_str()));
        update!(
            corner_radius,
            corner_radius_changed,
            i32::try_from(config.ui.corner_radius).unwrap_or(i32::MAX)
        );
        update!(
            tab_layout,
            tab_layout_changed,
            text(config.ui.tabs.as_str())
        );
        update!(
            animations_enabled,
            animations_enabled_changed,
            config.ui.animations.enable
        );
        update!(
            animation_speed,
            animation_speed_changed,
            config.ui.animations.speed
        );
        update!(
            theme_source,
            theme_source_changed,
            text(config.theme.source.as_str())
        );
        update!(theme_name, theme_name_changed, text(&config.theme.name));
        update!(
            error,
            error_changed,
            text(state.error.as_deref().unwrap_or(""))
        );
        update!(warnings, warnings_changed, warnings);

        for problem in state.error.iter().chain(&state.warnings) {
            eprintln!("ion: config: {problem}");
        }
        let revision = self.revision.wrapping_add(1);
        update!(revision, revision_changed, revision);
        self.as_mut().rust_mut().applied = Some(state);
    }

    fn value(&self, key: &QString) -> QVariant {
        ion_config::global()
            .get(&key.to_string())
            .map(|v| to_variant(&v))
            .unwrap_or_default()
    }

    fn set(self: Pin<&mut Self>, key: &QString, value: &QVariant) -> QString {
        let value = qobject::plain_variant(value);
        let result = match from_variant(&value) {
            Some(value) => ion_config::global().set(&key.to_string(), value),
            None => Err(format!("unsupported value type {}", value.type_name())),
        };
        self.finish(result)
    }

    fn reset(self: Pin<&mut Self>, key: &QString) -> QString {
        let result = ion_config::global().reset(&key.to_string());
        self.finish(result)
    }

    fn reload(self: Pin<&mut Self>) {
        let state = ion_config::global().reload();
        self.apply(state);
    }

    /// Apply a change right away, so QML reads the new value on the next line.
    fn finish(self: Pin<&mut Self>, result: Result<(), String>) -> QString {
        self.apply(ion_config::global().state());
        QString::from(result.err().unwrap_or_default().as_str())
    }

    fn overrides_as_nix(&self) -> QString {
        match ion_config::global().overrides_as_nix() {
            Ok(snippet) => QString::from(snippet.as_str()),
            Err(e) => QString::from(format!("# {e}").as_str()),
        }
    }
}

fn to_variant(value: &Value) -> QVariant {
    match value {
        Value::String(s) => QVariant::from(&QString::from(s.as_str())),
        Value::Integer(i) => QVariant::from(i),
        Value::Float(f) => QVariant::from(f),
        Value::Boolean(b) => QVariant::from(b),
        Value::Datetime(d) => QVariant::from(&QString::from(d.to_string().as_str())),
        Value::Array(items) => {
            let mut list = QList::<QVariant>::default();
            for item in items {
                list.append(to_variant(item));
            }
            QVariant::from(&list)
        }
        Value::Table(table) => {
            let mut map = QMap::<QMapPair_QString_QVariant>::default();
            for (k, v) in table {
                map.insert(QString::from(k.as_str()), to_variant(v));
            }
            QVariant::from(&map)
        }
    }
}

/// Convert what QML passed in. Numbers keep their QML type here; the store
/// matches them to the schema (whole numbers for integer settings).
fn from_variant(variant: &QVariant) -> Option<Value> {
    Some(match variant.type_name() {
        "bool" => Value::Boolean(variant.value::<bool>()?),
        "int" | "uint" | "qlonglong" | "qulonglong" | "short" | "ushort" => {
            Value::Integer(variant.value::<i64>()?)
        }
        "double" | "float" => Value::Float(variant.value::<f64>()?),
        "QString" | "QUrl" => Value::String(variant.value::<QString>()?.to_string()),
        "QStringList" => Value::Array(
            variant
                .value::<QStringList>()?
                .iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        ),
        "QVariantList" => Value::Array(
            variant
                .value::<QList<QVariant>>()?
                .iter()
                .map(from_variant)
                .collect::<Option<_>>()?,
        ),
        "QVariantMap" => Value::Table(
            variant
                .value::<QMap<QMapPair_QString_QVariant>>()?
                .iter()
                .map(|(k, v)| Some((k.to_string(), from_variant(v)?)))
                .collect::<Option<_>>()?,
        ),
        _ => return None,
    })
}
