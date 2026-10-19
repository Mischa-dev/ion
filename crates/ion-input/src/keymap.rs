//! Which keys run which command.
//!
//! Every command has default keys; `[shortcuts]` in the config replaces them
//! per command (`palette = "Ctrl+P"`, `reload = ["F5", "Ctrl+R"]`, or `""` to
//! unbind). Keys are Qt's portable key sequence text (`"Ctrl+Shift+T"`), where
//! `Ctrl` is Cmd on macOS. A key someone assigns is taken away from any
//! command that only had it by default, so remapping never leaves two
//! commands on one key (Qt would then run neither).

use std::collections::BTreeMap;

/// One command's keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The command id (`"new-tab"`).
    pub id: &'static str,
    /// Keys in effect, normalized; the first is shown as the hint.
    pub keys: Vec<String>,
    /// The command's own keys, normalized.
    pub defaults: Vec<String>,
    /// Whether `[shortcuts]` sets this command's keys.
    pub customized: bool,
}

/// All commands' keys after the config's remaps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Keymap {
    bindings: Vec<Binding>,
}

/// The `[shortcuts]` key for a command id: `"agent-sidebar"` → `"agentSidebar"`.
pub fn config_key(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    let mut upper = false;
    for c in id.chars() {
        if c == '-' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Modifier spellings people write, and Qt's portable name for each.
const MODIFIERS: &[(&str, &str)] = &[
    ("ctrl", "Ctrl"),
    ("control", "Ctrl"),
    ("cmd", "Ctrl"),
    ("command", "Ctrl"),
    ("alt", "Alt"),
    ("option", "Alt"),
    ("opt", "Alt"),
    ("shift", "Shift"),
    ("meta", "Meta"),
    ("super", "Meta"),
];

/// Qt's order for modifiers in portable text.
const MODIFIER_ORDER: &[&str] = &["Ctrl", "Alt", "Shift", "Meta"];

/// Named keys, lowercase alias first, Qt's portable name second.
const NAMED_KEYS: &[(&str, &str)] = &[
    ("left", "Left"),
    ("right", "Right"),
    ("up", "Up"),
    ("down", "Down"),
    ("pgup", "PgUp"),
    ("pageup", "PgUp"),
    ("pgdown", "PgDown"),
    ("pagedown", "PgDown"),
    ("home", "Home"),
    ("end", "End"),
    ("tab", "Tab"),
    ("backtab", "Backtab"),
    ("space", "Space"),
    ("esc", "Esc"),
    ("escape", "Esc"),
    ("return", "Return"),
    ("enter", "Enter"),
    ("backspace", "Backspace"),
    ("del", "Del"),
    ("delete", "Del"),
    ("ins", "Ins"),
    ("insert", "Ins"),
    ("print", "Print"),
    ("pause", "Pause"),
    ("menu", "Menu"),
    ("back", "Back"),
    ("forward", "Forward"),
    ("refresh", "Refresh"),
];

/// One key press (`"ctrl+shift+t"`) in Qt's portable spelling
/// (`"Ctrl+Shift+T"`), or `None` if it isn't a key Qt would accept.
fn normalize_press(press: &str) -> Option<String> {
    let mut rest = press.trim();
    let mut mods: Vec<&str> = Vec::new();
    // A modifier is a known word followed by `+` with something after it, so
    // "Ctrl++" is Ctrl and the plus key.
    while let Some(plus) = rest.find('+') {
        if plus == 0 || plus + 1 >= rest.len() {
            break;
        }
        let word = rest[..plus].trim().to_ascii_lowercase();
        let Some(&(_, name)) = MODIFIERS.iter().find(|(alias, _)| *alias == word) else {
            break;
        };
        if !mods.contains(&name) {
            mods.push(name);
        }
        rest = &rest[plus + 1..];
    }
    let key = rest.trim();
    let key = if key.is_empty() && rest.contains(' ') {
        "Space".to_owned()
    } else if key.chars().count() == 1 {
        key.to_uppercase()
    } else {
        let lower = key.to_ascii_lowercase();
        if let Some(&(_, name)) = NAMED_KEYS.iter().find(|(alias, _)| *alias == lower) {
            name.to_owned()
        } else if let Some(n) = lower.strip_prefix('f')
            && n.parse::<u8>().is_ok_and(|n| (1..=35).contains(&n))
        {
            format!("F{n}")
        } else {
            return None;
        }
    };
    if key.is_empty() {
        return None;
    }
    let mut out = String::new();
    for name in MODIFIER_ORDER.iter().filter(|m| mods.contains(m)) {
        out.push_str(name);
        out.push('+');
    }
    out.push_str(&key);
    Some(out)
}

/// A key sequence in Qt's portable spelling, so the same key always compares
/// equal: `"cmd+shift+t"` → `"Ctrl+Shift+T"`. Chords are comma separated
/// (`"Ctrl+K, Ctrl+B"`). `None` if any part isn't a key.
pub fn normalize(sequence: &str) -> Option<String> {
    let presses: Option<Vec<String>> = split_chord(sequence)
        .into_iter()
        .map(normalize_press)
        .collect();
    let presses = presses?;
    if presses.is_empty() || presses.len() > 4 {
        return None;
    }
    Some(presses.join(", "))
}

/// The presses of a chord. A comma right after `+` (or alone) is the comma key.
fn split_chord(sequence: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let bytes = sequence.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b',' && i > start && !sequence[start..i].trim().is_empty() {
            let before = sequence[start..i].trim_end();
            if before.ends_with('+') {
                continue;
            }
            parts.push(&sequence[start..i]);
            start = i + 1;
        }
    }
    if !sequence[start..].trim().is_empty() {
        parts.push(&sequence[start..]);
    }
    parts
}

impl Keymap {
    /// Build from each command's id and default keys, with `overrides`
    /// (`[shortcuts]`: config key or command id to keys) on top. Returns the
    /// keymap and a warning for each override that couldn't be used.
    pub fn new<'a>(
        commands: impl IntoIterator<Item = (&'static str, &'a [&'a str])>,
        overrides: &BTreeMap<String, Vec<String>>,
    ) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        let mut bindings: Vec<Binding> = commands
            .into_iter()
            .map(|(id, defaults)| {
                let defaults: Vec<String> =
                    defaults.iter().filter_map(|key| normalize(key)).collect();
                Binding {
                    id,
                    keys: defaults.clone(),
                    defaults,
                    customized: false,
                }
            })
            .collect();

        let mut taken: Vec<(String, &'static str)> = Vec::new();
        for (name, keys) in overrides {
            let Some(binding) = bindings
                .iter_mut()
                .find(|b| config_key(b.id) == *name || b.id == name)
            else {
                warnings.push(format!("shortcuts.{name}: no such command"));
                continue;
            };
            let mut chosen = Vec::new();
            for key in keys.iter().filter(|k| !k.trim().is_empty()) {
                let Some(key) = normalize(key) else {
                    warnings.push(format!("shortcuts.{name}: \"{key}\" isn't a key"));
                    continue;
                };
                if let Some((_, owner)) = taken.iter().find(|(k, _)| *k == key) {
                    warnings.push(format!(
                        "shortcuts.{name}: {key} is already set for {}",
                        config_key(owner)
                    ));
                    continue;
                }
                if !chosen.contains(&key) {
                    taken.push((key.clone(), binding.id));
                    chosen.push(key);
                }
            }
            binding.keys = chosen;
            binding.customized = true;
        }

        // Keys someone assigned win over the same key's default elsewhere.
        for binding in bindings.iter_mut().filter(|b| !b.customized) {
            binding.keys.retain(|key| {
                !taken
                    .iter()
                    .any(|(k, owner)| k == key && *owner != binding.id)
            });
        }
        (Self { bindings }, warnings)
    }

    /// Every command's binding, in command order.
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// The keys that run `id`.
    pub fn keys(&self, id: &str) -> &[String] {
        self.binding(id).map_or(&[], |b| b.keys.as_slice())
    }

    pub fn binding(&self, id: &str) -> Option<&Binding> {
        self.bindings.iter().find(|b| b.id == id)
    }

    /// The command `key` runs, if any.
    pub fn owner(&self, key: &str) -> Option<&'static str> {
        let key = normalize(key)?;
        self.bindings
            .iter()
            .find(|b| b.keys.contains(&key))
            .map(|b| b.id)
    }

    /// The commands whose keys change when `id` gets exactly `keys`: `id`
    /// itself, then any command that loses one of them. Keys that don't
    /// parse are dropped.
    pub fn assign(&self, id: &str, keys: &[String]) -> Vec<(&'static str, Vec<String>)> {
        let Some(target) = self.binding(id) else {
            return Vec::new();
        };
        let mut keys: Vec<String> = keys.iter().filter_map(|k| normalize(k)).collect();
        keys.dedup();
        let mut changes = vec![(target.id, keys.clone())];
        for binding in self.bindings.iter().filter(|b| b.id != id) {
            if binding.keys.iter().any(|k| keys.contains(k)) {
                let left = binding
                    .keys
                    .iter()
                    .filter(|k| !keys.contains(k))
                    .cloned()
                    .collect();
                changes.push((binding.id, left));
            }
        }
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMANDS: &[(&str, &[&str])] = &[
        ("new-tab", &["Ctrl+T"]),
        ("reload", &["F5", "Ctrl+R"]),
        ("palette", &["Ctrl+K"]),
        ("agent-sidebar", &["Ctrl+Shift+E"]),
    ];

    fn keymap(overrides: &[(&str, &[&str])]) -> (Keymap, Vec<String>) {
        let table = overrides
            .iter()
            .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
            .collect();
        Keymap::new(COMMANDS.iter().copied(), &table)
    }

    #[test]
    fn config_keys_are_camel_case() {
        assert_eq!(config_key("agent-sidebar"), "agentSidebar");
        assert_eq!(config_key("palette"), "palette");
        assert_eq!(config_key("tab-1"), "tab1");
    }

    #[test]
    fn normalizes_spellings() {
        assert_eq!(normalize("cmd+shift+t").as_deref(), Some("Ctrl+Shift+T"));
        assert_eq!(normalize("Shift+Ctrl+t").as_deref(), Some("Ctrl+Shift+T"));
        assert_eq!(normalize("Ctrl++").as_deref(), Some("Ctrl++"));
        assert_eq!(normalize("Ctrl+Shift+.").as_deref(), Some("Ctrl+Shift+."));
        assert_eq!(normalize("ctrl+pagedown").as_deref(), Some("Ctrl+PgDown"));
        assert_eq!(normalize("f11").as_deref(), Some("F11"));
        assert_eq!(normalize("option+left").as_deref(), Some("Alt+Left"));
        assert_eq!(
            normalize("ctrl+k, ctrl+b").as_deref(),
            Some("Ctrl+K, Ctrl+B")
        );
        assert_eq!(normalize("Ctrl+,").as_deref(), Some("Ctrl+,"));
        assert_eq!(normalize("Ctrl+Hyper"), None);
        assert_eq!(normalize(""), None);
        assert_eq!(normalize("F99"), None);
    }

    #[test]
    fn defaults_apply_without_overrides() {
        let (map, warnings) = keymap(&[]);
        assert!(warnings.is_empty());
        assert_eq!(map.keys("reload"), ["F5", "Ctrl+R"]);
        assert_eq!(map.owner("ctrl+k"), Some("palette"));
        assert!(!map.binding("reload").unwrap().customized);
    }

    #[test]
    fn overrides_replace_and_unbind() {
        let (map, _) = keymap(&[("palette", &["Ctrl+P"]), ("reload", &[""])]);
        assert_eq!(map.keys("palette"), ["Ctrl+P"]);
        assert!(map.keys("reload").is_empty());
        assert_eq!(map.owner("Ctrl+K"), None);
    }

    #[test]
    fn overrides_accept_ids_and_config_keys() {
        let (map, warnings) = keymap(&[("agent-sidebar", &["F2"]), ("newTab", &["F3"])]);
        assert!(warnings.is_empty());
        assert_eq!(map.keys("agent-sidebar"), ["F2"]);
        assert_eq!(map.keys("new-tab"), ["F3"]);
    }

    #[test]
    fn an_assigned_key_leaves_its_default_owner() {
        let (map, _) = keymap(&[("palette", &["Ctrl+R"])]);
        assert_eq!(map.keys("reload"), ["F5"]);
        assert_eq!(map.owner("Ctrl+R"), Some("palette"));
    }

    #[test]
    fn two_overrides_on_one_key_keep_the_first() {
        let (map, warnings) = keymap(&[("palette", &["F2"]), ("reload", &["F2"])]);
        assert_eq!(map.keys("palette"), ["F2"]);
        assert!(map.keys("reload").is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn bad_overrides_warn() {
        let (_, warnings) = keymap(&[("nope", &["F2"]), ("reload", &["Ctrl+Nope"])]);
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn assign_takes_the_key_from_its_owner() {
        let (map, _) = keymap(&[]);
        let changes = map.assign("palette", &["ctrl+r".into(), "nope+".into()]);
        assert_eq!(
            changes,
            [
                ("palette", vec!["Ctrl+R".to_string()]),
                ("reload", vec!["F5".to_string()]),
            ]
        );
        assert!(map.assign("nope", &[]).is_empty());
    }
}
