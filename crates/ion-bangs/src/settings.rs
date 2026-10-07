//! Settings the palette can change in one step, such as "Theme: Nord" or
//! "Vertical tabs".
//!
//! Each option is a set of config keys to write. Choosing it saves them as
//! overrides through `Config.set`, so they apply live and persist; the base
//! config (and so the Nix config) is left untouched.

use ion_config::{Config, Density, TabLayout, ThemeSource};

/// A value to store under a config key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingValue {
    Text(String),
    Flag(bool),
}

/// One choosable setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingOption {
    pub title: String,
    /// Extra words people might search for.
    pub keywords: &'static str,
    /// Config keys and the values to write, in order.
    pub changes: Vec<(&'static str, SettingValue)>,
    /// Whether the config already has these values.
    pub active: bool,
}

/// Search engines offered as one-step choices.
const SEARCH_ENGINES: &[(&str, &str)] = &[
    ("DuckDuckGo", "https://duckduckgo.com/?q={}"),
    ("Google", "https://www.google.com/search?q={}"),
    ("Brave Search", "https://search.brave.com/search?q={}"),
    ("Kagi", "https://kagi.com/search?q={}"),
    ("Startpage", "https://www.startpage.com/do/search?query={}"),
    ("Ecosia", "https://www.ecosia.org/search?q={}"),
];

fn text(value: &str) -> SettingValue {
    SettingValue::Text(value.to_owned())
}

/// "catppuccin-mocha" → "Catppuccin Mocha".
fn display_name(id: &str) -> String {
    id.split(['-', '_'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every option, given the current config and the built-in theme ids.
pub fn options<'a>(
    config: &Config,
    themes: impl IntoIterator<Item = &'a str>,
) -> Vec<SettingOption> {
    let mut options = Vec::new();
    let builtin = config.theme.source == ThemeSource::Builtin;

    options.push(SettingOption {
        title: "Theme: Automatic light or dark".into(),
        keywords: "settings appearance colors system mode",
        changes: vec![
            ("theme.source", text("builtin")),
            ("theme.name", text("auto")),
        ],
        active: builtin && config.theme.name == "auto",
    });
    for id in themes {
        options.push(SettingOption {
            title: format!("Theme: {}", display_name(id)),
            keywords: "settings appearance colors palette",
            changes: vec![("theme.source", text("builtin")), ("theme.name", text(id))],
            active: builtin && config.theme.name == id,
        });
    }
    options.push(SettingOption {
        title: "Theme: Follow system accent".into(),
        keywords: "settings appearance colors macos",
        changes: vec![("theme.source", text("system"))],
        active: config.theme.source == ThemeSource::System,
    });
    options.push(SettingOption {
        title: "Theme: Follow DankMaterialShell".into(),
        keywords: "settings appearance colors dms matugen linux",
        changes: vec![("theme.source", text("dms"))],
        active: config.theme.source == ThemeSource::Dms,
    });

    for (density, title, value) in [
        (Density::Comfortable, "Comfortable density", "comfortable"),
        (Density::Compact, "Compact density", "compact"),
    ] {
        options.push(SettingOption {
            title: title.into(),
            keywords: "settings layout size spacing",
            changes: vec![("ui.density", text(value))],
            active: config.ui.density == density,
        });
    }

    for (layout, title, value) in [
        (TabLayout::Horizontal, "Horizontal tabs", "horizontal"),
        (TabLayout::Vertical, "Vertical tabs", "vertical"),
    ] {
        options.push(SettingOption {
            title: title.into(),
            keywords: "settings layout sidebar tab strip",
            changes: vec![("ui.tabs", text(value))],
            active: config.ui.tabs == layout,
        });
    }

    let animations = config.ui.animations.enable;
    options.push(SettingOption {
        title: if animations {
            "Turn off animations"
        } else {
            "Turn on animations"
        }
        .into(),
        keywords: "settings motion reduce",
        changes: vec![("ui.animations.enable", SettingValue::Flag(!animations))],
        active: false,
    });

    let restore = config.general.restore_session;
    options.push(SettingOption {
        title: if restore {
            "Start with a new tab instead of the last session"
        } else {
            "Reopen the last session on start"
        }
        .into(),
        keywords: "settings startup restore tabs",
        changes: vec![("general.restoreSession", SettingValue::Flag(!restore))],
        active: false,
    });

    for &(name, template) in SEARCH_ENGINES {
        options.push(SettingOption {
            title: format!("Search with {name}"),
            keywords: "settings default search engine",
            changes: vec![
                ("search.engine", text(name)),
                ("search.template", text(template)),
            ],
            active: config.search.template == template,
        });
    }

    options
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(options: &[SettingOption]) -> Vec<&str> {
        options.iter().map(|o| o.title.as_str()).collect()
    }

    #[test]
    fn theme_names_read_nicely() {
        assert_eq!(display_name("catppuccin-mocha"), "Catppuccin Mocha");
        assert_eq!(display_name("nord"), "Nord");
        assert_eq!(display_name("ion-dark"), "Ion Dark");
    }

    #[test]
    fn defaults_mark_the_current_choices() {
        let options = options(&Config::default(), ["ion-dark", "nord"]);
        let active: Vec<_> = options
            .iter()
            .filter(|o| o.active)
            .map(|o| o.title.as_str())
            .collect();
        assert_eq!(
            active,
            [
                "Theme: Automatic light or dark",
                "Comfortable density",
                "Horizontal tabs",
                "Search with DuckDuckGo",
            ]
        );
    }

    #[test]
    fn picking_a_theme_also_selects_builtin_themes() {
        let options = options(&Config::default(), ["nord"]);
        let nord = options.iter().find(|o| o.title == "Theme: Nord").unwrap();
        assert_eq!(
            nord.changes,
            [
                ("theme.source", text("builtin")),
                ("theme.name", text("nord"))
            ]
        );
    }

    #[test]
    fn toggles_offer_the_opposite_of_the_current_value() {
        let mut config = Config::default();
        assert!(titles(&options(&config, [])).contains(&"Turn off animations"));
        config.ui.animations.enable = false;
        let options = options(&config, []);
        let on = options
            .iter()
            .find(|o| o.title == "Turn on animations")
            .unwrap();
        assert_eq!(
            on.changes,
            [("ui.animations.enable", SettingValue::Flag(true))]
        );
    }

    #[test]
    fn search_engine_templates_have_a_query_slot() {
        for &(name, template) in SEARCH_ENGINES {
            assert!(template.contains("{}"), "{name}");
        }
    }
}
