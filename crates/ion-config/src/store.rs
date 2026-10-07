//! The live config: loaded from disk, edited through the overrides file,
//! reloaded when either file changes, with subscribers told about each change.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Duration;

use notify::{RecursiveMode, Watcher as _};
use toml::{Table, Value};
use toml_edit::{DocumentMut, Item, TableLike};

use crate::layer::{self, Loaded};
use crate::schema::Config;

/// Where the two config files live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub dir: PathBuf,
    /// Written by Nix (home-manager) or by hand. Ion never writes it.
    pub base: PathBuf,
    /// Written by Ion's settings UI. Wins over the base file.
    pub overrides: PathBuf,
}

impl Paths {
    pub fn in_dir(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self {
            base: dir.join("config.toml"),
            overrides: dir.join("overrides.toml"),
            dir,
        }
    }

    /// `$ION_CONFIG_DIR`, else `$XDG_CONFIG_HOME/ion`, else `~/.config/ion`.
    /// The same on macOS, so one home-manager module covers both platforms.
    pub fn from_env() -> Option<Self> {
        let var = |name: &str| {
            std::env::var_os(name)
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
        };
        let dir = var("ION_CONFIG_DIR")
            .or_else(|| var("XDG_CONFIG_HOME").map(|d| d.join("ion")))
            .or_else(|| var("HOME").map(|d| d.join(".config").join("ion")))?;
        Some(Self::in_dir(dir))
    }
}

/// One consistent view of the config and its diagnostics.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct State {
    pub config: Arc<Config>,
    /// Unknown keys and semantic problems; the config is still in use.
    pub warnings: Vec<String>,
    /// Why the files on disk could not be used. While set, `config` is the
    /// last config that loaded (or the defaults at startup).
    pub error: Option<String>,
}

type Callback = Box<dyn Fn(&Arc<State>) + Send + Sync>;

struct Inner {
    paths: Option<Paths>,
    state: RwLock<Arc<State>>,
    subscribers: Mutex<Vec<(u64, Callback)>>,
    next_id: AtomicU64,
    /// Serializes reloads and writes so they apply in order.
    io: Mutex<()>,
}

/// A shared handle to the live config. Cheap to clone.
#[derive(Clone)]
pub struct Store {
    inner: Arc<Inner>,
}

impl Store {
    /// Load the config from `paths`. Never fails: problems end up in
    /// [`State::error`] and the defaults are used.
    pub fn open(paths: Paths) -> Self {
        let store = Self::new(Some(paths));
        store.reload();
        store
    }

    /// Defaults only, with no files behind them. `set` and `reset` fail.
    pub fn in_memory() -> Self {
        Self::new(None)
    }

    fn new(paths: Option<Paths>) -> Self {
        Self {
            inner: Arc::new(Inner {
                paths,
                state: RwLock::default(),
                subscribers: Mutex::default(),
                next_id: AtomicU64::new(0),
                io: Mutex::default(),
            }),
        }
    }

    pub fn paths(&self) -> Option<&Paths> {
        self.inner.paths.as_ref()
    }

    pub fn state(&self) -> Arc<State> {
        self.inner
            .state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn config(&self) -> Arc<Config> {
        self.state().config.clone()
    }

    /// The effective value of a dotted key (`ui.cornerRadius`), defaults
    /// included. `None` for keys the schema does not have.
    pub fn get(&self, key: &str) -> Option<Value> {
        let table = Table::try_from(&*self.config()).ok()?;
        layer::lookup(&table, key).cloned()
    }

    /// Re-read both files and notify subscribers if anything changed.
    pub fn reload(&self) -> Arc<State> {
        let _io = self.inner.io.lock().unwrap_or_else(|e| e.into_inner());
        self.reload_locked()
    }

    fn reload_locked(&self) -> Arc<State> {
        let previous = self.state();
        let next = match self.paths().map(load) {
            None => State::default(),
            Some(Ok(Loaded { config, warnings })) => State {
                config: Arc::new(config),
                warnings,
                error: None,
            },
            Some(Err(error)) => State {
                config: previous.config.clone(),
                warnings: previous.warnings.clone(),
                error: Some(error),
            },
        };
        if *previous == next {
            return previous;
        }
        let next = Arc::new(next);
        *self.inner.state.write().unwrap_or_else(|e| e.into_inner()) = next.clone();
        let subscribers = self
            .inner
            .subscribers
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        for (_, callback) in subscribers.iter() {
            callback(&next);
        }
        next
    }

    /// Set `key` in the overrides file. Rejected (and nothing written) if the
    /// result would not load.
    pub fn set(&self, key: &str, value: Value) -> Result<(), String> {
        let parts = layer::key_path(key)?;
        let value = coerce(self.get(key).as_ref(), value);
        self.edit(|doc| {
            let (last, parents) = parts.split_last().expect("key_path is never empty");
            let mut table: &mut dyn TableLike = doc.as_table_mut();
            for part in parents {
                if table.get(part).is_none() {
                    let mut child = toml_edit::Table::new();
                    child.set_implicit(true);
                    table.insert(part, Item::Table(child));
                }
                table = table
                    .get_mut(part)
                    .and_then(Item::as_table_like_mut)
                    .ok_or_else(|| format!("{key}: {part} is not a table"))?;
            }
            let item = match edit_value(value) {
                // A whole table (`bangs`) reads better as a `[section]`.
                toml_edit::Value::InlineTable(t) => Item::Table(t.into_table()),
                value => Item::Value(value),
            };
            table.insert(last, item);
            Ok(())
        })
    }

    /// Remove `key` from the overrides file, so the base file or default
    /// applies again.
    pub fn reset(&self, key: &str) -> Result<(), String> {
        let parts = layer::key_path(key)?;
        self.edit(|doc| {
            remove(doc.as_table_mut(), &parts);
            Ok(())
        })
    }

    /// The overrides as a `programs.ion` Nix snippet ("promote to Nix").
    pub fn overrides_as_nix(&self) -> Result<String, String> {
        let paths = self.paths().ok_or("no config files")?;
        Ok(crate::nix::snippet(&layer::read_table(&paths.overrides)?))
    }

    fn edit(
        &self,
        change: impl FnOnce(&mut DocumentMut) -> Result<(), String>,
    ) -> Result<(), String> {
        let paths = self.paths().ok_or("config is not backed by files")?;
        let _io = self.inner.io.lock().unwrap_or_else(|e| e.into_inner());

        let text = match std::fs::read_to_string(&paths.overrides) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(format!("{}: {e}", paths.overrides.display())),
        };
        let mut doc: DocumentMut = text.parse().map_err(|e: toml_edit::TomlError| {
            format!("{}: {}", paths.overrides.display(), e.message())
        })?;
        change(&mut doc)?;
        let text = doc.to_string().trim_start().to_owned();

        // Only write what would load.
        let mut merged = layer::read_table(&paths.base)?;
        layer::merge(&mut merged, layer::parse_table(&text)?);
        layer::to_config(merged)?;

        write_atomically(&paths.overrides, &text)
            .map_err(|e| format!("{}: {e}", paths.overrides.display()))?;
        self.reload_locked();
        Ok(())
    }

    /// Call `callback` (on whichever thread reloads) after every change.
    /// The subscription ends when the returned guard is dropped.
    pub fn subscribe(
        &self,
        callback: impl Fn(&Arc<State>) + Send + Sync + 'static,
    ) -> Subscription {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        self.inner
            .subscribers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((id, Box::new(callback)));
        Subscription {
            store: Arc::downgrade(&self.inner),
            id,
        }
    }

    /// Reload whenever either file changes on disk, until the returned
    /// watcher is dropped. Creates the config directory if needed.
    pub fn watch(&self) -> Result<Watcher, String> {
        let paths = self.paths().ok_or("config is not backed by files")?.clone();
        std::fs::create_dir_all(&paths.dir).map_err(|e| format!("{}: {e}", paths.dir.display()))?;

        let (tx, rx) = mpsc::channel::<()>();
        let names: Vec<std::ffi::OsString> = [&paths.base, &paths.overrides]
            .iter()
            .filter_map(|p| p.file_name().map(ToOwned::to_owned))
            .collect();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let Ok(event) = event else { return };
                if event.kind.is_access() {
                    return;
                }
                if event
                    .paths
                    .iter()
                    .any(|p| p.file_name().is_some_and(|n| names.iter().any(|m| m == n)))
                {
                    let _ = tx.send(());
                }
            })
            .map_err(|e| e.to_string())?;
        watcher
            .watch(&paths.dir, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;

        // Editors and home-manager touch files in bursts; reload once it settles.
        let store = Arc::downgrade(&self.inner);
        std::thread::Builder::new()
            .name("ion-config-watch".into())
            .spawn(move || {
                while rx.recv().is_ok() {
                    while rx.recv_timeout(Duration::from_millis(50)).is_ok() {}
                    let Some(inner) = store.upgrade() else { break };
                    Store { inner }.reload();
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Watcher { _watcher: watcher })
    }
}

/// Ends a [`Store::subscribe`] subscription when dropped.
pub struct Subscription {
    store: Weak<Inner>,
    id: u64,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(inner) = self.store.upgrade() {
            inner
                .subscribers
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .retain(|(id, _)| *id != self.id);
        }
    }
}

/// Keeps a [`Store::watch`] running while alive.
pub struct Watcher {
    _watcher: notify::RecommendedWatcher,
}

fn load(paths: &Paths) -> Result<Loaded, String> {
    let mut table = layer::read_table(&paths.base)?;
    layer::merge(&mut table, layer::read_table(&paths.overrides)?);
    layer::to_config(table)
}

fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

/// Remove `parts` from `table`, dropping tables the removal leaves empty.
fn remove(table: &mut dyn TableLike, parts: &[&str]) {
    match parts {
        [] => {}
        [last] => {
            table.remove(last);
        }
        [first, rest @ ..] => {
            let Some(child) = table.get_mut(first).and_then(Item::as_table_like_mut) else {
                return;
            };
            remove(child, rest);
            if child.is_empty() {
                table.remove(first);
            } else if let Some(Item::Table(child)) = table.get_mut(first) {
                // Drop a `[ui]` header left with only subtables under it.
                if child.iter().all(|(_, item)| item.is_table()) {
                    child.set_implicit(true);
                }
            }
        }
    }
}

/// Match number kinds to the schema, so a UI that only has "numbers"
/// (QML/JavaScript) writes `speed = 2.0` and `cornerRadius = 12`.
fn coerce(current: Option<&Value>, value: Value) -> Value {
    match (current, value) {
        (Some(Value::Float(_)), Value::Integer(i)) => Value::Float(i as f64),
        (Some(Value::Integer(_)), Value::Float(f))
            if f.fract() == 0.0 && f.abs() < 2f64.powi(53) =>
        {
            Value::Integer(f as i64)
        }
        (_, value) => value,
    }
}

fn edit_value(value: Value) -> toml_edit::Value {
    match value {
        Value::String(s) => s.into(),
        Value::Integer(i) => i.into(),
        Value::Float(f) => f.into(),
        Value::Boolean(b) => b.into(),
        Value::Datetime(d) => d
            .to_string()
            .parse()
            .unwrap_or_else(|_| d.to_string().into()),
        Value::Array(items) => items
            .into_iter()
            .map(edit_value)
            .collect::<toml_edit::Array>()
            .into(),
        Value::Table(t) => t
            .into_iter()
            .map(|(k, v)| (k, edit_value(v)))
            .collect::<toml_edit::InlineTable>()
            .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Density, TabLayout};

    fn store_with(base: &str) -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::in_dir(dir.path());
        std::fs::write(&paths.base, base).unwrap();
        (dir, Store::open(paths))
    }

    #[test]
    fn missing_files_mean_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(Paths::in_dir(dir.path().join("nothing-here")));
        assert_eq!(*store.config(), Config::default());
        assert_eq!(store.state().error, None);
    }

    #[test]
    fn set_writes_overrides_and_keeps_comments() {
        let (_dir, store) = store_with("[ui]\ndensity = \"compact\"\n");
        let overrides = &store.paths().unwrap().overrides;
        std::fs::write(overrides, "# my tweaks\n[ui]\ncornerRadius = 2\n").unwrap();
        store.reload();

        store
            .set("ui.tabs", Value::String("vertical".into()))
            .unwrap();
        store.set("ui.animations.speed", Value::Float(2.0)).unwrap();
        store
            .set(
                "bangs.gh",
                Value::String("https://github.com/search?q={}".into()),
            )
            .unwrap();

        let config = store.config();
        assert_eq!(config.ui.density, Density::Compact);
        assert_eq!(config.ui.tabs, TabLayout::Vertical);
        assert_eq!(config.ui.corner_radius, 2);
        assert_eq!(config.ui.animations.speed, 2.0);
        let text = std::fs::read_to_string(overrides).unwrap();
        assert!(text.starts_with("# my tweaks\n"), "{text}");
        assert_eq!(store.get("ui.tabs"), Some(Value::String("vertical".into())));
    }

    #[test]
    fn numbers_follow_the_schema() {
        let (_dir, store) = store_with("");
        store.set("ui.animations.speed", Value::Integer(2)).unwrap();
        store.set("ui.cornerRadius", Value::Float(12.0)).unwrap();
        let text = std::fs::read_to_string(&store.paths().unwrap().overrides).unwrap();
        assert!(text.contains("speed = 2.0"), "{text}");
        assert!(text.contains("cornerRadius = 12\n"), "{text}");
        assert!(store.set("ui.cornerRadius", Value::Float(1.5)).is_err());
    }

    #[test]
    fn invalid_set_is_rejected_and_not_written() {
        let (_dir, store) = store_with("");
        assert!(
            store
                .set("ui.density", Value::String("huge".into()))
                .is_err()
        );
        assert!(
            store
                .set("ui.cornerRadius", Value::String("big".into()))
                .is_err()
        );
        assert!(!store.paths().unwrap().overrides.exists());
        assert!(store.set("", Value::Boolean(true)).is_err());
    }

    #[test]
    fn reset_removes_the_override_and_empty_tables() {
        let (_dir, store) = store_with("[ui]\ncornerRadius = 4\n");
        store.set("ui.cornerRadius", Value::Integer(16)).unwrap();
        assert_eq!(store.config().ui.corner_radius, 16);
        store.reset("ui.cornerRadius").unwrap();
        assert_eq!(store.config().ui.corner_radius, 4);
        let text = std::fs::read_to_string(&store.paths().unwrap().overrides).unwrap();
        assert_eq!(text.trim(), "");
    }

    #[test]
    fn tables_are_written_as_sections() {
        let (_dir, store) = store_with("");
        store.set("ui.cornerRadius", Value::Integer(1)).unwrap();
        store.set("ui.animations.speed", Value::Float(1.5)).unwrap();
        let bangs: Table = "gh = \"https://github.com/search?q={}\"".parse().unwrap();
        store.set("bangs", Value::Table(bangs)).unwrap();
        store.reset("ui.cornerRadius").unwrap();
        let text = std::fs::read_to_string(&store.paths().unwrap().overrides).unwrap();
        assert_eq!(
            text,
            "[ui.animations]\nspeed = 1.5\n\n[bangs]\ngh = \"https://github.com/search?q={}\"\n"
        );
        assert_eq!(store.config().bangs["gh"], "https://github.com/search?q={}");
    }

    #[test]
    fn broken_file_keeps_last_good_config() {
        let (_dir, store) = store_with("[ui]\ncornerRadius = 4\n");
        let base = store.paths().unwrap().base.clone();
        std::fs::write(&base, "[ui\n").unwrap();
        let state = store.reload();
        assert!(state.error.as_deref().unwrap().contains("config.toml"));
        assert_eq!(state.config.ui.corner_radius, 4);
        std::fs::write(&base, "[ui]\ncornerRadius = 6\n").unwrap();
        let state = store.reload();
        assert_eq!(state.error, None);
        assert_eq!(state.config.ui.corner_radius, 6);
    }

    #[test]
    fn subscribers_hear_changes_until_dropped() {
        let (_dir, store) = store_with("");
        let (tx, rx) = mpsc::channel();
        let sub = store.subscribe(move |state| tx.send(state.config.ui.corner_radius).unwrap());
        store.set("ui.cornerRadius", Value::Integer(3)).unwrap();
        assert_eq!(rx.try_recv(), Ok(3));
        store.reload(); // nothing changed, so no notification
        assert!(rx.try_recv().is_err());
        drop(sub);
        store.set("ui.cornerRadius", Value::Integer(5)).unwrap();
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn watch_reloads_on_external_edits() {
        let (_dir, store) = store_with("");
        let (tx, rx) = mpsc::channel();
        let _sub = store.subscribe(move |state| {
            let _ = tx.send(state.config.ui.corner_radius);
        });
        let _watcher = store.watch().unwrap();
        std::fs::write(&store.paths().unwrap().base, "[ui]\ncornerRadius = 9\n").unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            match rx.recv_timeout(left) {
                Ok(9) => break,
                Ok(_) => continue,
                Err(e) => panic!("no reload after editing config.toml: {e}"),
            }
        }
    }

    #[test]
    fn overrides_render_as_nix() {
        let (_dir, store) = store_with("");
        store.set("ui.cornerRadius", Value::Integer(12)).unwrap();
        assert_eq!(
            store.overrides_as_nix().unwrap(),
            "programs.ion = {\n  ui.cornerRadius = 12;\n};\n"
        );
    }

    #[test]
    fn in_memory_is_read_only() {
        let store = Store::in_memory();
        assert_eq!(store.get("ui.cornerRadius"), Some(Value::Integer(8)));
        assert!(store.set("ui.cornerRadius", Value::Integer(1)).is_err());
    }
}
