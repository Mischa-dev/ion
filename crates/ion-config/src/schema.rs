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
    pub downloads: Downloads,
    pub new_tab: NewTab,
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
    pub agents: Agents,
    pub safety: Safety,
    pub ai: Ai,
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
    /// With vertical tabs, shrink the sidebar to favicons; it expands while
    /// the pointer is over it.
    pub collapse_sidebar: bool,
    pub animations: Animations,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            density: Density::default(),
            corner_radius: 8,
            tabs: TabLayout::default(),
            collapse_sidebar: false,
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
    /// Give page scrollbars, form controls and text selection Ion's colors,
    /// unless the page styles them itself.
    pub page_controls: bool,
    /// Per-site theming, keyed by site (`example.com` also covers its
    /// subdomains; the most specific entry wins).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub sites: BTreeMap<String, SiteTheme>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            source: ThemeSource::default(),
            name: "auto".into(),
            palette: None,
            pages: PageTheming::default(),
            page_controls: true,
            sites: BTreeMap::new(),
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

/// Theming for one site.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SiteTheme {
    /// Darken the site under a dark theme (`true`) or never (`false`); unset
    /// follows `theme.pages`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub darken: Option<bool>,
    /// CSS added to the site's pages.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub css: String,
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

/// `[agents]`: Ion's MCP server, and each agent's trust under its id
/// (`[agents.ion]`, `[agents.claude-code]`). Agents not listed ask before
/// everything. `ion-safety` turns these into its agent profiles; see
/// docs/SAFETY.md.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Agents {
    pub mcp: Mcp,
    /// Agent id to its settings. `mcp` is reserved for the section above.
    #[serde(flatten)]
    pub profiles: BTreeMap<String, Agent>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Mcp {
    /// Serve Ion's browser tools to outside agents over localhost MCP.
    pub enable: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Agent {
    /// Name shown in prompts and the activity log; empty uses the id.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub trust: AgentTrust,
    /// Sites a `trustedSites` agent acts on freely: `"github.com"` (and its
    /// subdomains) or `"https://github.com"` (that origin only).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trusted_sites: Vec<String>,
    /// Connectors this agent may use without asking.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connectors: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<AgentRule>,
    /// The model this agent runs on, which makes it one you can ask with
    /// `@<id>` from the address bar. Empty for a trust profile only (an
    /// outside agent, or Ion Agent, which runs on `[ai]`).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub model: String,
    /// The model service's base URL; empty uses `ai.baseUrl`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub base_url: String,
    /// The environment variable holding its API key; unset uses
    /// `ai.apiKeyEnv`, empty sends none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    /// Extra instructions for the agent, added to its system prompt.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub instructions: String,
}

/// How much an agent may do without asking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentTrust {
    #[default]
    Ask,
    TrustedSites,
    Full,
    Custom,
}

/// One `[[agents.<id>.rules]]` entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRule {
    /// An action id (`"submit"`), `"tier:<tier>"`, or `"*"`.
    pub action: String,
    /// A domain, an origin, or `"*"`.
    #[serde(default = "any_site")]
    pub site: String,
    pub effect: RuleEffect,
}

fn any_site() -> String {
    "*".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleEffect {
    Allow,
    Ask,
    Deny,
}

/// `[safety]`: the permission model's own settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Safety {
    /// Days of agent activity log kept (at least 1).
    pub audit_retention_days: u32,
}

impl Default for Safety {
    fn default() -> Self {
        Self {
            audit_retention_days: 30,
        }
    }
}

/// `[ai]`: Ion Agent and the model it runs on. Off by default while agents
/// are new: with `enable = false` there is no agent UI and nothing starts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Ai {
    /// Turn on Ion Agent: `@ion …` and `!ai …` in the address bar.
    pub enable: bool,
    /// Which kind of model service: `"openai"` is any OpenAI-compatible
    /// chat completions API (OpenAI, OpenRouter, Ollama, llama.cpp).
    pub provider: String,
    /// The model id sent to the service.
    pub model: String,
    /// The service's base URL, up to and including `/v1`.
    pub base_url: String,
    /// The environment variable that holds the API key. The key itself
    /// never goes in config. Empty sends no key (local servers).
    pub api_key_env: String,
}

impl Default for Ai {
    fn default() -> Self {
        Self {
            enable: false,
            provider: "openai".into(),
            model: "gpt-5-mini".into(),
            base_url: "https://api.openai.com/v1".into(),
            api_key_env: "OPENAI_API_KEY".into(),
        }
    }
}

/// Where downloads are saved.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Downloads {
    /// Folder for downloads; empty uses the system's download folder.
    /// A leading `~` means the home directory.
    pub directory: String,
    /// Per-type folders: `document`, `image`, `audio`, `video`, `archive` or
    /// `other` to a folder. Types not listed go to `directory`.
    pub folders: BTreeMap<String, String>,
}

/// The new-tab page's site tiles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NewTab {
    /// Tiles always shown first, in this order.
    pub shortcuts: Vec<NewTabShortcut>,
    /// Fill the remaining tiles with the sites visited most.
    pub most_visited: bool,
    /// How many tiles to show at most.
    pub tiles: u32,
}

impl Default for NewTab {
    fn default() -> Self {
        Self {
            shortcuts: Vec::new(),
            most_visited: true,
            tiles: 8,
        }
    }
}

/// One pinned new-tab tile.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NewTabShortcut {
    pub title: String,
    pub url: String,
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
as_str!(AgentTrust { Ask => "ask", TrustedSites => "trustedSites", Full => "full", Custom => "custom" });
as_str!(RuleEffect { Allow => "allow", Ask => "ask", Deny => "deny" });
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
        if self.agents.profiles.contains_key("mcp") {
            problems.push("agents.mcp is Ion's MCP server settings, not an agent".into());
        }
        if self.ai.provider != "openai" {
            problems.push(format!(
                "ai.provider {:?} isn't supported yet; use \"openai\" (any OpenAI-compatible API)",
                self.ai.provider
            ));
        }
        if self.safety.audit_retention_days == 0 {
            problems.push("safety.auditRetentionDays must be at least 1".into());
        }
        for (i, shortcut) in self.new_tab.shortcuts.iter().enumerate() {
            if shortcut.url.trim().is_empty() {
                problems.push(format!("newTab.shortcuts[{i}] has no url"));
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

            [theme.sites."example.com"]
            darken = false
            css = "body { max-width: 50em }"

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
            "#,
        )
        .unwrap();
        assert_eq!(config.theme.source, ThemeSource::Dms);
        assert_eq!(config.theme.sites["example.com"].darken, Some(false));
        assert_eq!(
            config.theme.sites["example.com"].css,
            "body { max-width: 50em }"
        );
        assert_eq!(config.ui.density, Density::Compact);
        assert_eq!(config.ui.tabs, TabLayout::Vertical);
        assert_eq!(config.bangs["gh"], "https://github.com/search?q={}");
        assert_eq!(config.general, General::default());
        assert_eq!(config.sites["example.com"].javascript, Some(false));
        assert!(!config.privacy.block_third_party_cookies);
        assert!(config.privacy.global_privacy_control);
    }

    #[test]
    fn new_tab_shortcuts_parse() {
        let config: Config = toml::from_str(
            r#"
            [newTab]
            mostVisited = false
            tiles = 4
            shortcuts = [
                { title = "Mail", url = "https://mail.example" },
                { url = "" },
            ]
            "#,
        )
        .unwrap();
        assert!(!config.new_tab.most_visited);
        assert_eq!(config.new_tab.tiles, 4);
        assert_eq!(config.new_tab.shortcuts[0].title, "Mail");
        assert_eq!(config.check(), ["newTab.shortcuts[1] has no url"]);
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
    fn custom_agents_name_their_own_model() {
        let config: Config = toml::from_str(
            r#"
            [agents.claude]
            name = "Claude"
            model = "claude-sonnet-5-5"
            baseUrl = "https://api.anthropic.com/v1/"
            apiKeyEnv = "ANTHROPIC_API_KEY"
            instructions = "Be brief."

            [agents.claude-code]
            trust = "full"
            "#,
        )
        .unwrap();
        let claude = &config.agents.profiles["claude"];
        assert_eq!(claude.model, "claude-sonnet-5-5");
        assert_eq!(claude.api_key_env.as_deref(), Some("ANTHROPIC_API_KEY"));
        assert_eq!(claude.instructions, "Be brief.");
        let outside = &config.agents.profiles["claude-code"];
        assert!(outside.model.is_empty());
        assert_eq!(outside.api_key_env, None);
    }

    #[test]
    fn agents_parse_beside_the_mcp_section() {
        let config: Config = toml::from_str(
            r#"
            [agents.mcp]
            enable = true

            [agents.claude-code]
            trust = "full"

            [agents.ion]
            trust = "trustedSites"
            trustedSites = ["github.com"]
            connectors = ["github"]

            [[agents.ion.rules]]
            action = "submit"
            site = "example.com"
            effect = "deny"

            [[agents.ion.rules]]
            action = "tier:read"
            effect = "allow"

            [safety]
            auditRetentionDays = 7
            "#,
        )
        .unwrap();
        assert!(config.agents.mcp.enable);
        assert_eq!(config.agents.profiles.len(), 2);
        assert_eq!(
            config.agents.profiles["claude-code"].trust,
            AgentTrust::Full
        );
        let ion = &config.agents.profiles["ion"];
        assert_eq!(ion.trust, AgentTrust::TrustedSites);
        assert_eq!(ion.trusted_sites, ["github.com"]);
        assert_eq!(ion.rules[0].effect, RuleEffect::Deny);
        assert_eq!(ion.rules[1].site, "*");
        assert_eq!(config.safety.audit_retention_days, 7);
        assert!(config.check().is_empty());
        let text = toml::to_string(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
    }

    #[test]
    fn agent_enums_match_serde() {
        for trust in [
            AgentTrust::Ask,
            AgentTrust::TrustedSites,
            AgentTrust::Full,
            AgentTrust::Custom,
        ] {
            assert_eq!(
                toml::Value::try_from(trust).unwrap().as_str(),
                Some(trust.as_str())
            );
        }
        for effect in [RuleEffect::Allow, RuleEffect::Ask, RuleEffect::Deny] {
            assert_eq!(
                toml::Value::try_from(effect).unwrap().as_str(),
                Some(effect.as_str())
            );
        }
    }

    #[test]
    fn ai_is_off_until_turned_on() {
        let config = Config::default();
        assert!(!config.ai.enable);
        assert!(config.check().is_empty());

        let config: Config = toml::from_str(
            r#"
            [ai]
            enable = true
            model = "qwen3:8b"
            baseUrl = "http://127.0.0.1:11434/v1"
            apiKeyEnv = ""
            "#,
        )
        .unwrap();
        assert!(config.ai.enable);
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "qwen3:8b");
        assert_eq!(config.ai.api_key_env, "");

        let config: Config = toml::from_str("[ai]\nprovider = \"chatgpt\"").unwrap();
        assert_eq!(config.check().len(), 1);
    }

    #[test]
    fn round_trips_through_toml() {
        let config = Config::default();
        let text = toml::to_string(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
    }
}
