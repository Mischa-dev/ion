//! The typed shape of Ion's configuration.
//!
//! Keys are camelCase so the TOML on disk reads exactly like the
//! `programs.ion` Nix options (`ui.cornerRadius` in both places). Every field
//! has a default, so an empty file is a valid config.
//!
//! Each section is read by the workstream that owns the feature; adding a
//! field here (with a default) never breaks an existing config file.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The whole configuration, after layering and defaults.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub general: General,
    pub search: Search,
    pub ui: Ui,
    pub theme: Theme,
    /// User-defined bangs: name (without `!`) to URL template with `{}`.
    /// Ion's built-in bangs are merged in by `ion-bangs`; entries here win.
    pub bangs: BTreeMap<String, String>,
    pub adblock: Adblock,
    /// Shortcut remaps: command id (for example `"palette"`) to a Qt key
    /// sequence (for example `"Ctrl+K"`). Unlisted commands keep their default.
    pub shortcuts: BTreeMap<String, String>,
    /// Per-site settings, keyed by host. `"example.com"` also covers its
    /// subdomains; the most specific key wins. `"*"` covers every site.
    pub sites: BTreeMap<String, Site>,
    pub privacy: Privacy,
    pub keyboard: Keyboard,
    /// Unpacked Chrome extension folders (Manifest V3) loaded at startup,
    /// on top of those in Ion's own extensions folder.
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Keyboard {
    /// Vim-style keys in pages: j/k to scroll, f for link hints, H/L for
    /// back and forward.
    pub vim: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Privacy {
    /// Tell sites not to sell or share your data: the `Sec-GPC: 1` header
    /// and `navigator.globalPrivacyControl`.
    pub global_privacy_control: bool,
    /// Refuse cookies set by sites other than the one in the address bar.
    pub block_third_party_cookies: bool,
}

impl Default for Privacy {
    fn default() -> Self {
        Self {
            global_privacy_control: true,
            block_third_party_cookies: true,
        }
    }
}

/// Settings for one site. Unset fields fall back to less specific keys.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Site {
    /// Run JavaScript on the site.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub javascript: Option<bool>,
    /// Style sheet added to the site's pages.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub css: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct General {
    /// Page opened at startup when there is nothing to restore.
    pub home_page: String,
    /// Reopen the previous session's tabs on start.
    pub restore_session: bool,
    /// Unload background tabs not looked at for this many minutes, to save
    /// memory; they reload when shown. 0 never unloads tabs.
    pub suspend_tabs_after: u32,
}

impl Default for General {
    fn default() -> Self {
        Self {
            home_page: "https://duckduckgo.com/".into(),
            restore_session: true,
            suspend_tabs_after: 30,
        }
    }
}

/// The engine used when URL-bar input is not an address.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Search {
    pub engine: String,
    /// Search URL with `{}` where the query goes.
    pub template: String,
}

impl Default for Search {
    fn default() -> Self {
        Self {
            engine: "DuckDuckGo".into(),
            template: "https://duckduckgo.com/?q={}".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Ui {
    pub density: Density,
    /// Corner radius in logical pixels, from 0 (sharp) up.
    pub corner_radius: u32,
    pub tabs: TabLayout,
    pub animations: Animations,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            density: Density::default(),
            corner_radius: 8,
            tabs: TabLayout::default(),
            animations: Animations::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    #[default]
    Comfortable,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TabLayout {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Animations {
    pub enable: bool,
    /// Multiplier on animation speed: 2.0 is twice as fast, 0.5 half as fast.
    pub speed: f64,
}

impl Default for Animations {
    fn default() -> Self {
        Self {
            enable: true,
            speed: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Theme {
    pub source: ThemeSource,
    /// Theme id to use with `source = "builtin"`; `"auto"` follows the
    /// system's light/dark mode.
    pub name: String,
    /// Palette file for `source = "manual"` or `"dms"` (DMS has a default path
    /// of its own when this is unset).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub palette: Option<PathBuf>,
    /// What web pages see of the theme.
    pub pages: PageTheming,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            source: ThemeSource::default(),
            name: "auto".into(),
            palette: None,
            pages: PageTheming::default(),
        }
    }
}

/// Where Ion's palette comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeSource {
    /// Curated themes shipped with Ion.
    #[default]
    Builtin,
    /// DankMaterialShell's matugen-generated palette file (Linux).
    Dms,
    /// System accent color and light/dark mode.
    System,
    /// A palette file named by `theme.palette` (default
    /// `<config dir>/palette.toml`).
    Manual,
}

/// How web pages follow the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageTheming {
    /// Pages get the theme's light/dark as `prefers-color-scheme`.
    #[default]
    Match,
    /// Pages get the system's light/dark setting.
    System,
    /// Like `match`; with a dark theme, pages without a dark style of their
    /// own are darkened too.
    Darken,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Adblock {
    pub enable: bool,
    /// Filter lists: well-known names (`easylist`, `easyprivacy`,
    /// `ublock-filters`, …) or URLs.
    pub lists: Vec<String>,
}

impl Default for Adblock {
    fn default() -> Self {
        Self {
            enable: true,
            lists: vec![
                "easylist".into(),
                "easyprivacy".into(),
                "ublock-filters".into(),
            ],
        }
    }
}

/// The lowercase word each enum value has in the TOML file.
macro_rules! as_str {
    ($ty:ty { $($variant:ident => $word:literal),* $(,)? }) => {
        impl $ty {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $word),*
                }
            }
        }
    };
}

as_str!(Density { Comfortable => "comfortable", Compact => "compact" });
as_str!(TabLayout { Horizontal => "horizontal", Vertical => "vertical" });
as_str!(ThemeSource { Builtin => "builtin", Dms => "dms", System => "system", Manual => "manual" });
as_str!(PageTheming { Match => "match", System => "system", Darken => "darken" });

impl Config {
    /// Problems serde cannot express, as human-readable messages. The config
    /// is still used when there are some; they are shown as warnings.
    pub fn check(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if !self.search.template.contains("{}") {
            problems.push(format!(
                "search.template {:?} has no {{}} for the query",
                self.search.template
            ));
        }
        if !(self.ui.animations.speed.is_finite() && self.ui.animations.speed > 0.0) {
            problems.push(format!(
                "ui.animations.speed must be above 0, got {}",
                self.ui.animations.speed
            ));
        }
        for (name, template) in &self.bangs {
            // An empty template removes a built-in bang.
            if !template.is_empty() && !template.contains("{}") {
                problems.push(format!(
                    "bangs.{name} {template:?} has no {{}} for the query"
                ));
            }
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_is_all_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config, Config::default());
        assert!(config.check().is_empty());
    }

    #[test]
    fn spec_example_parses() {
        // The TOML that the `programs.ion` example in the product spec produces.
        let config: Config = toml::from_str(
            r#"
            [theme]
            source = "dms"

            [ui]
            density = "compact"
            cornerRadius = 8
            tabs = "vertical"
            animations = { enable = true, speed = 1.0 }

            [bangs]
            gh = "https://github.com/search?q={}"
            nix = "https://search.nixos.org/packages?query={}"

            [adblock]
            lists = ["easylist", "easyprivacy", "ublock-filters"]

            [privacy]
            blockThirdPartyCookies = false

            [sites."example.com"]
            javascript = false
            css = "body { font-size: 120% }"
            "#,
        )
        .unwrap();
        assert_eq!(config.theme.source, ThemeSource::Dms);
        assert_eq!(config.ui.density, Density::Compact);
        assert_eq!(config.ui.tabs, TabLayout::Vertical);
        assert_eq!(config.bangs["gh"], "https://github.com/search?q={}");
        assert_eq!(config.general, General::default());
        assert_eq!(config.sites["example.com"].javascript, Some(false));
        assert!(!config.privacy.block_third_party_cookies);
        assert!(config.privacy.global_privacy_control);
    }

    #[test]
    fn check_reports_semantic_problems() {
        let mut config = Config::default();
        config.search.template = "https://example.com/".into();
        config.ui.animations.speed = 0.0;
        // Manual without a palette is fine: it falls back to
        // `<config dir>/palette.toml`.
        config.theme.source = ThemeSource::Manual;
        assert_eq!(config.check().len(), 2);
    }

    #[test]
    fn as_str_matches_serde() {
        for source in [
            ThemeSource::Builtin,
            ThemeSource::Dms,
            ThemeSource::System,
            ThemeSource::Manual,
        ] {
            assert_eq!(
                toml::Value::try_from(source).unwrap().as_str(),
                Some(source.as_str())
            );
        }
        for density in [Density::Comfortable, Density::Compact] {
            assert_eq!(
                toml::Value::try_from(density).unwrap().as_str(),
                Some(density.as_str())
            );
        }
        for tabs in [TabLayout::Horizontal, TabLayout::Vertical] {
            assert_eq!(
                toml::Value::try_from(tabs).unwrap().as_str(),
                Some(tabs.as_str())
            );
        }
        for pages in [PageTheming::Match, PageTheming::System, PageTheming::Darken] {
            assert_eq!(
                toml::Value::try_from(pages).unwrap().as_str(),
                Some(pages.as_str())
            );
        }
    }

    #[test]
    fn round_trips_through_toml() {
        let config = Config::default();
        let text = toml::to_string(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
    }
}
