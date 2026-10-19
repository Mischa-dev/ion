//! What Ion's Settings page (`ion://settings`) shows.
//!
//! The page is drawn from data: [`sections`] lists every section, each a few
//! groups of settings, and each setting names a config key and the control
//! that edits it. The QML page renders whatever is here, so a feature adds
//! its settings by adding a section file, not by editing the page:
//!
//! 1. `src/sections/<name>.rs` with `pub fn section(ctx: &Context) -> Section`.
//! 2. One line for it in `src/sections/mod.rs`, where the order is the
//!    sidebar's order.
//!
//! A setting that needs more than the stock controls uses
//! [`Control::Custom`], naming a QML file in `crates/ion-app/qml/settings/`
//! that draws it. [`Control::Action`] is a button the page handles.
//!
//! Changes are written by `Config.set` to `overrides.toml`, never to
//! `config.toml`, so a config managed by home-manager keeps working: Nix owns
//! the base file and the page's changes layer on top (docs/CONFIG.md).

pub mod sections;

use ion_config::{Origin, Store, Value};
use serde::Serialize;

/// What sections may depend on, filled in by the app.
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Theme ids Ion can use (`ion-dark`, `nord`, installed palette files…).
    pub themes: Vec<String>,
}

/// One entry in the Settings sidebar.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Section {
    /// Used in links: `ion://settings/<id>`.
    pub id: String,
    pub title: String,
    /// The icon beside the title: a name from `qml/components/IconSet.qml`.
    pub icon: String,
    /// One line under the title at the top of the section.
    pub summary: String,
    pub groups: Vec<Group>,
}

/// A card of related settings within a section.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Group {
    /// Empty for a group without a heading.
    pub title: String,
    pub settings: Vec<Setting>,
}

/// One row on the page.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Setting {
    /// The dotted config key it edits (`ui.cornerRadius`). For an action or a
    /// custom control that edits nothing, a unique id.
    pub key: String,
    pub label: String,
    /// A sentence under the label; empty for none.
    pub help: String,
    /// Extra words that find the setting in search.
    pub keywords: String,
    pub control: Control,
    /// A boolean key that must be on for this row to be enabled
    /// (`ai.model` needs `ai.enable`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires: Option<String>,
}

/// How a setting is edited.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Control {
    /// On or off.
    Toggle,
    /// One value out of a few, as a segmented control or a menu.
    Choice {
        options: Vec<Choice>,
    },
    /// One of a few named sets of values for several keys at once, such as
    /// a search engine's name and URL. `key` is only the row's id.
    Preset {
        options: Vec<Preset>,
    },
    Text {
        placeholder: String,
    },
    /// A whole number or decimal, shown as a slider when `slider` is set.
    Number {
        min: f64,
        max: f64,
        step: f64,
        /// Shown after the value ("px", "min").
        unit: String,
        slider: bool,
    },
    /// A list of strings, edited as removable chips plus an entry field.
    List {
        placeholder: String,
    },
    /// A list of values from a fixed set, as checkboxes.
    Flags {
        options: Vec<Choice>,
    },
    /// A folder path, typed or picked.
    Folder {
        placeholder: String,
    },
    /// A button that runs `action` on the page (see SettingsPage.qml).
    Action {
        action: String,
        button: String,
    },
    /// Drawn by `crates/ion-app/qml/settings/<component>.qml`, which gets
    /// the setting as `setting`.
    Custom {
        component: String,
    },
}

/// One value a [`Control::Choice`] or [`Control::Flags`] offers.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Choice {
    pub value: String,
    pub label: String,
}

/// One option of a [`Control::Preset`]: the keys it sets, and their values.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Preset {
    pub label: String,
    pub values: Vec<(String, Value)>,
}

impl Setting {
    pub fn new(key: &str, label: &str, control: Control) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            help: String::new(),
            keywords: String::new(),
            control,
            requires: None,
        }
    }

    pub fn help(mut self, help: &str) -> Self {
        self.help = help.into();
        self
    }

    pub fn keywords(mut self, keywords: &str) -> Self {
        self.keywords = keywords.into();
        self
    }

    pub fn requires(mut self, key: &str) -> Self {
        self.requires = Some(key.into());
        self
    }

    /// Whether every word of `query` appears in this setting's text or in
    /// its section's title.
    fn matches(&self, section: &Section, words: &[String]) -> bool {
        let mut haystack = format!(
            "{} {} {} {} {}",
            self.label, self.help, self.keywords, self.key, section.title
        );
        if let Control::Choice { options } | Control::Flags { options } = &self.control {
            for option in options {
                haystack.push(' ');
                haystack.push_str(&option.label);
            }
        }
        if let Control::Preset { options } = &self.control {
            for option in options {
                haystack.push(' ');
                haystack.push_str(&option.label);
            }
        }
        let haystack = haystack.to_lowercase();
        words.iter().all(|word| haystack.contains(word.as_str()))
    }
}

impl Group {
    pub fn new(title: &str, settings: Vec<Setting>) -> Self {
        Self {
            title: title.into(),
            settings,
        }
    }
}

impl Section {
    pub fn new(id: &str, title: &str, icon: &str, summary: &str, groups: Vec<Group>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: icon.into(),
            summary: summary.into(),
            groups,
        }
    }

    pub fn settings(&self) -> impl Iterator<Item = &Setting> {
        self.groups.iter().flat_map(|g| &g.settings)
    }
}

/// Shorthand for a list of choices: `choices(&[("compact", "Compact")])`.
pub fn choices(items: &[(&str, &str)]) -> Vec<Choice> {
    items
        .iter()
        .map(|&(value, label)| Choice {
            value: value.into(),
            label: label.into(),
        })
        .collect()
}

/// Every section, in sidebar order.
pub fn sections(ctx: &Context) -> Vec<Section> {
    sections::all(ctx)
}

/// The sections and settings matching `query`, each with only its matching
/// settings and empty groups dropped. A blank query keeps everything.
pub fn search(sections: Vec<Section>, query: &str) -> Vec<Section> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return sections;
    }
    sections
        .into_iter()
        .filter_map(|mut section| {
            let probe = section.clone();
            for group in &mut section.groups {
                group.settings.retain(|s| s.matches(&probe, &words));
            }
            section.groups.retain(|g| !g.settings.is_empty());
            (!section.groups.is_empty()).then_some(section)
        })
        .collect()
}

/// A setting with its current state, as the page shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SettingState {
    #[serde(flatten)]
    pub setting: Setting,
    /// The effective value; `null` for actions and unknown keys.
    pub value: serde_json::Value,
    /// Ion's default.
    pub default: serde_json::Value,
    /// "default", "base" (config.toml, e.g. from Nix) or "override" (set here).
    pub origin: &'static str,
    /// What config.toml says, when it sets the key; shown when the page's
    /// change hides it.
    pub base: serde_json::Value,
    /// For a preset: the index of the option the config matches, else -1.
    pub selected: i64,
}

/// [`SettingState`]s for every setting in `sections`, grouped as they are.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SectionState {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub summary: String,
    pub groups: Vec<GroupState>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GroupState {
    pub title: String,
    pub settings: Vec<SettingState>,
}

/// Attach current values from `store` to each setting.
pub fn with_state(sections: Vec<Section>, store: &Store) -> Vec<SectionState> {
    let defaults = toml::Table::try_from(ion_config::Config::default()).unwrap_or_default();
    let json = |value: Option<Value>| {
        value
            .and_then(|v| serde_json::to_value(v).ok())
            .unwrap_or(serde_json::Value::Null)
    };
    sections
        .into_iter()
        .map(|section| SectionState {
            id: section.id,
            title: section.title,
            icon: section.icon,
            summary: section.summary,
            groups: section
                .groups
                .into_iter()
                .map(|group| GroupState {
                    title: group.title,
                    settings: group
                        .settings
                        .into_iter()
                        .map(|setting| {
                            let (origin, selected) = match &setting.control {
                                Control::Preset { options } => preset_state(options, store),
                                _ => (store.origin(&setting.key), -1),
                            };
                            SettingState {
                                value: json(store.get(&setting.key)),
                                default: json(lookup(&defaults, &setting.key).cloned()),
                                origin: origin.as_str(),
                                base: json(store.base_value(&setting.key)),
                                selected,
                                setting,
                            }
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

/// Which preset the config matches, and where its keys come from (the
/// strongest layer that sets any of them).
fn preset_state(options: &[Preset], store: &Store) -> (Origin, i64) {
    let selected = options.iter().position(|option| {
        option
            .values
            .iter()
            .all(|(key, value)| store.get(key).as_ref() == Some(value))
    });
    let keys = options.iter().flat_map(|o| o.values.iter().map(|(k, _)| k));
    let mut origin = Origin::Default;
    for key in keys {
        match store.origin(key) {
            Origin::Override => origin = Origin::Override,
            Origin::Base if origin == Origin::Default => origin = Origin::Base,
            _ => {}
        }
    }
    (origin, selected.map_or(-1, |i| i as i64))
}

fn lookup<'a>(table: &'a toml::Table, key: &str) -> Option<&'a Value> {
    let mut parts = key.split('.').peekable();
    let mut current = table;
    while let Some(part) = parts.next() {
        let value = current.get(part)?;
        if parts.peek().is_none() {
            return Some(value);
        }
        current = value.as_table()?;
    }
    None
}

/// The page's data as JSON: `with_state(search(sections(ctx), query))`.
pub fn page_json(ctx: &Context, query: &str, store: &Store) -> String {
    let sections = with_state(search(sections(ctx), query), store);
    serde_json::to_string(&sections).unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        Context {
            themes: vec!["ion-dark".into(), "nord".into()],
        }
    }

    /// Every key a stock control edits exists in the config schema, so a
    /// renamed config field can't leave a dead row behind.
    #[test]
    fn every_key_is_a_config_key() {
        let store = Store::in_memory();
        for section in sections(&ctx()) {
            for setting in section.settings() {
                match &setting.control {
                    Control::Action { .. } | Control::Custom { .. } => {}
                    Control::Preset { options } => {
                        for (key, _) in options.iter().flat_map(|o| &o.values) {
                            assert!(store.get(key).is_some(), "{}: {key}", section.id);
                        }
                    }
                    _ => assert!(
                        store.get(&setting.key).is_some(),
                        "{}: unknown key {}",
                        section.id,
                        setting.key
                    ),
                }
                if let Some(key) = &setting.requires {
                    assert!(
                        matches!(store.get(key), Some(Value::Boolean(_))),
                        "{key} is not a switch"
                    );
                }
            }
        }
    }

    #[test]
    fn ids_and_keys_are_unique() {
        let all = sections(&ctx());
        let mut ids: Vec<&str> = all.iter().map(|s| s.id.as_str()).collect();
        let mut keys: Vec<&str> = all
            .iter()
            .flat_map(|s| s.settings())
            .map(|s| s.key.as_str())
            .collect();
        let (id_count, key_count) = (ids.len(), keys.len());
        ids.sort_unstable();
        ids.dedup();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(ids.len(), id_count, "duplicate section id");
        assert_eq!(keys.len(), key_count, "duplicate setting key");
    }

    #[test]
    fn choices_are_values_the_config_accepts() {
        let dir = std::env::temp_dir().join(format!("ion-settings-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open(ion_config::Paths::in_dir(&dir));
        for section in sections(&ctx()) {
            for setting in section.settings() {
                if let Control::Choice { options } = &setting.control {
                    // Themes are only checked by the theme engine.
                    if setting.key == "theme.name" {
                        continue;
                    }
                    for option in options {
                        store
                            .set(&setting.key, Value::String(option.value.clone()))
                            .unwrap_or_else(|e| panic!("{}={}: {e}", setting.key, option.value));
                    }
                    store.reset(&setting.key).unwrap();
                }
            }
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn search_keeps_matching_rows_only() {
        let found = search(sections(&ctx()), "vertical TABS");
        let keys: Vec<&str> = found
            .iter()
            .flat_map(|s| s.settings())
            .map(|s| s.key.as_str())
            .collect();
        assert!(keys.contains(&"ui.tabs"), "{keys:?}");
        assert!(!keys.contains(&"privacy.globalPrivacyControl"));
        assert!(search(sections(&ctx()), "zzzz-nothing").is_empty());
        assert_eq!(search(sections(&ctx()), "  ").len(), sections(&ctx()).len());
    }

    #[test]
    fn search_finds_choice_labels_and_section_titles() {
        let found = search(sections(&ctx()), "nord");
        assert!(
            found
                .iter()
                .flat_map(|s| s.settings())
                .any(|s| s.key == "theme.name")
        );
        let found = search(sections(&ctx()), "kagi");
        assert!(
            found
                .iter()
                .flat_map(|s| s.settings())
                .any(|s| s.key == "search.engine")
        );
    }

    #[test]
    fn state_reports_values_defaults_and_presets() {
        let store = Store::in_memory();
        let page = with_state(sections(&ctx()), &store);
        let all: Vec<&SettingState> = page
            .iter()
            .flat_map(|s| &s.groups)
            .flat_map(|g| &g.settings)
            .collect();
        let radius = all
            .iter()
            .find(|s| s.setting.key == "ui.cornerRadius")
            .unwrap();
        assert_eq!(radius.value, serde_json::json!(8));
        assert_eq!(radius.default, serde_json::json!(8));
        assert_eq!(radius.origin, "default");
        let engine = all
            .iter()
            .find(|s| s.setting.key == "search.engine")
            .unwrap();
        assert_eq!(engine.selected, 0, "DuckDuckGo is the default");
        let json = page_json(&ctx(), "", &store);
        assert!(json.starts_with('[') && json.contains("\"kind\":\"toggle\""));
    }
}
