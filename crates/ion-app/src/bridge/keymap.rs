//! `Keymap` QML singleton: which keys run which command
//! (`ion_bangs::COMMANDS` with `[shortcuts]` from the config on top, through
//! `ion_input::Keymap`), the shortcut editor's edits, and the commands run
//! lately for the palette.
//!
//! Results follow the config: read them in bindings that also mention
//! `Config.revision`.

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
        #[namespace = "ion"]
        type Keymap = super::KeymapRust;

        /// Every command as `{ id, title, keywords, keys, defaults,
        /// customized, hidden }`, in command order; `keys` and `defaults` are
        /// lists of portable key text.
        #[qinvokable]
        fn bindings(self: &Keymap) -> QVariant;

        /// The keys that run `id`, for a Shortcut's `sequences`.
        #[qinvokable]
        fn keys(self: &Keymap, id: &QString) -> QStringList;

        /// The command `sequence` runs, or "".
        #[qinvokable]
        fn owner(self: &Keymap, sequence: &QString) -> QString;

        /// A command's title, or "".
        #[qinvokable]
        fn title(self: &Keymap, id: &QString) -> QString;

        /// Portable key text the way this platform writes it ("⇧⌘T" on macOS).
        #[qinvokable]
        fn display(self: &Keymap, sequence: &QString) -> QString;

        /// A key event's `key` and `modifiers` as portable key text, or ""
        /// while only modifiers are held.
        #[qinvokable]
        #[cxx_name = "fromEvent"]
        fn key_for_event(self: &Keymap, key: i32, modifiers: i32) -> QString;

        /// Give `id` exactly `keys`, taking them from whichever commands had
        /// them, and save that to the config. Returns an error or "".
        #[qinvokable]
        fn assign(self: Pin<&mut Keymap>, id: &QString, keys: &QStringList) -> QString;

        /// Put `id` back on its default keys. Returns an error or "".
        #[qinvokable]
        fn reset(self: Pin<&mut Keymap>, id: &QString) -> QString;

        /// Put every command back on its default keys. Returns an error or "".
        #[qinvokable]
        #[cxx_name = "resetAll"]
        fn reset_all(self: Pin<&mut Keymap>) -> QString;

        /// Problems with `[shortcuts]` in the config.
        #[qinvokable]
        fn warnings(self: &Keymap) -> QStringList;

        /// Note that `id` just ran, for the palette's recent commands.
        #[qinvokable]
        #[cxx_name = "noteUsed"]
        fn note_used(self: Pin<&mut Keymap>, id: &QString);
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/keys.h");

        #[cxx_name = "keySequenceFromEvent"]
        fn key_sequence_from_event(key: i32, modifiers: i32) -> QString;

        #[cxx_name = "keySequenceNativeText"]
        fn key_sequence_native_text(portable: &QString) -> QString;
    }
}

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QStringList, QVariant};
use ion_bangs::COMMANDS;
use ion_config::{Config, Value};
use ion_input::Keymap;
use ion_input::keymap::config_key;

#[derive(Default)]
pub struct KeymapRust;

/// The keymap for `config`.
fn build(config: &Config) -> (Keymap, Vec<String>) {
    let overrides: BTreeMap<String, Vec<String>> = config
        .shortcuts
        .iter()
        .map(|(name, keys)| (name.clone(), keys.to_vec()))
        .collect();
    Keymap::new(COMMANDS.iter().map(|c| (c.id, c.keys)), &overrides)
}

/// A keymap and the problems found building it.
type Built = Arc<(Keymap, Vec<String>)>;

/// The keymap for the current config, rebuilt only when the config changes.
/// Problems are printed once per config.
pub fn current() -> Built {
    static CACHE: Mutex<Option<(Arc<Config>, Built)>> = Mutex::new(None);
    let config = ion_config::global().config();
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((seen, keymap)) = cache.as_ref()
        && Arc::ptr_eq(seen, &config)
    {
        return keymap.clone();
    }
    let keymap = Arc::new(build(&config));
    for warning in &keymap.1 {
        eprintln!("ion: config: {warning}");
    }
    *cache = Some((config, keymap.clone()));
    keymap
}

/// Commands run lately, most recent first, kept in `recent-commands.txt` in
/// Ion's data directory.
static RECENT: Mutex<Option<Vec<String>>> = Mutex::new(None);

fn recent_path() -> Option<PathBuf> {
    Some(ion_session::paths::data_dir()?.join("recent-commands.txt"))
}

/// The commands run lately, most recent first.
pub fn recent() -> Vec<String> {
    let mut recent = RECENT.lock().unwrap_or_else(|e| e.into_inner());
    recent
        .get_or_insert_with(|| {
            recent_path()
                .and_then(|path| std::fs::read_to_string(path).ok())
                .map(|text| text.lines().map(str::to_owned).collect())
                .unwrap_or_default()
        })
        .clone()
}

fn strings(list: &QStringList) -> Vec<String> {
    QList::<QString>::from(list)
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn string_list(items: &[String]) -> QStringList {
    let mut list = QList::<QString>::default();
    for item in items {
        list.append(QString::from(item.as_str()));
    }
    QStringList::from(&list)
}

/// Save `keys` for `id` in the overrides file, or drop the override when they
/// are the defaults (and no other config layer sets them).
fn save(id: &'static str, keys: &[String]) -> Result<(), String> {
    let store = ion_config::global();
    let key = format!("shortcuts.{}", config_key(id));
    let defaults = current()
        .0
        .binding(id)
        .map(|b| b.defaults.clone())
        .unwrap_or_default();
    if keys == defaults.as_slice() {
        // An override spelled with the command id is the same setting.
        if config_key(id) != id {
            store.reset(&format!("shortcuts.{id}"))?;
        }
        store.reset(&key)?;
        if current().0.keys(id) == keys {
            return Ok(());
        }
    }
    let value = match keys {
        [one] => Value::String(one.clone()),
        _ => Value::Array(keys.iter().cloned().map(Value::String).collect()),
    };
    store.set(&key, value)
}

fn result(outcome: Result<(), String>) -> QString {
    QString::from(outcome.err().unwrap_or_default().as_str())
}

impl qobject::Keymap {
    fn bindings(&self) -> QVariant {
        let keymap = current();
        let mut rows = QList::<QVariant>::default();
        for (command, binding) in COMMANDS.iter().zip(keymap.0.bindings()) {
            let mut row = QMap::<QMapPair_QString_QVariant>::default();
            let mut set = |key: &str, value: QVariant| row.insert(QString::from(key), value);
            set("id", QVariant::from(&QString::from(command.id)));
            set("title", QVariant::from(&QString::from(command.title)));
            set("keywords", QVariant::from(&QString::from(command.keywords)));
            set("keys", QVariant::from(&string_list(&binding.keys)));
            set("defaults", QVariant::from(&string_list(&binding.defaults)));
            set("customized", QVariant::from(&binding.customized));
            set("hidden", QVariant::from(&command.hidden));
            rows.append(QVariant::from(&row));
        }
        QVariant::from(&rows)
    }

    fn keys(&self, id: &QString) -> QStringList {
        string_list(current().0.keys(&id.to_string()))
    }

    fn owner(&self, sequence: &QString) -> QString {
        QString::from(current().0.owner(&sequence.to_string()).unwrap_or_default())
    }

    fn title(&self, id: &QString) -> QString {
        QString::from(ion_bangs::commands::find(&id.to_string()).map_or("", |c| c.title))
    }

    fn display(&self, sequence: &QString) -> QString {
        qobject::key_sequence_native_text(sequence)
    }

    fn key_for_event(&self, key: i32, modifiers: i32) -> QString {
        qobject::key_sequence_from_event(key, modifiers)
    }

    fn assign(self: Pin<&mut Self>, id: &QString, keys: &QStringList) -> QString {
        let changes = current().0.assign(&id.to_string(), &strings(keys));
        if changes.is_empty() {
            return QString::from("no such command");
        }
        result(changes.iter().try_for_each(|(id, keys)| save(id, keys)))
    }

    fn reset(self: Pin<&mut Self>, id: &QString) -> QString {
        let id = id.to_string();
        let Some(binding) = current().0.binding(&id).cloned() else {
            return QString::from("no such command");
        };
        // Its default keys may belong to other commands by now.
        let changes = current().0.assign(&id, &binding.defaults);
        result(changes.iter().try_for_each(|(id, keys)| save(id, keys)))
    }

    fn reset_all(self: Pin<&mut Self>) -> QString {
        let store = ion_config::global();
        let names: Vec<String> = store.config().shortcuts.keys().cloned().collect();
        result(
            names
                .iter()
                .try_for_each(|name| store.reset(&format!("shortcuts.{name}"))),
        )
    }

    fn warnings(&self) -> QStringList {
        string_list(&current().1)
    }

    fn note_used(self: Pin<&mut Self>, id: &QString) {
        let mut list = recent();
        ion_bangs::commands::remember(&mut list, &id.to_string());
        if let Some(path) = recent_path() {
            let text: String = list.iter().map(|id| format!("{id}\n")).collect();
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = std::fs::write(&path, text) {
                eprintln!("ion: {}: {e}", path.display());
            }
        }
        *RECENT.lock().unwrap_or_else(|e| e.into_inner()) = Some(list);
    }
}
