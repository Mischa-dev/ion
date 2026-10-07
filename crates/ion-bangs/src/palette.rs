//! Ranking for the Ctrl/Cmd+K command palette.
//!
//! One query searches open tabs, browsing history, saved sessions, Ion
//! commands, settings and bangs together, plus a "open / search for what I typed" entry
//! resolved by the URL bar's [`Omnibox`]. Two prefixes narrow it down:
//!
//! - `>` lists commands and settings only.
//! - `!` lists bangs while the trigger is being typed; once a known bang is
//!   followed by a query, the only result is the expanded search.
//!
//! History arrives already filtered and ordered by the history store; the
//! palette only re-ranks it against everything else.

use std::sync::Arc;

use ion_core::navigation::{Omnibox, Resolved};

use crate::bang::BangTable;
use crate::commands::{COMMANDS, Command};
use crate::fuzzy;
use crate::settings::{SettingOption, SettingValue};

/// Most results returned for one query.
pub const MAX_RESULTS: usize = 50;
/// Most suggestions shown under the URL bar.
pub const SUGGESTIONS: usize = 6;

/// Points a bang loses against a tab or command with the same match quality.
const BANG_PENALTY: i32 = 2;
/// Points a history page loses against an open tab with the same match quality.
const HISTORY_PENALTY: i32 = 1;

/// An open tab or a history page, as the palette sees it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TabEntry {
    pub title: String,
    pub url: String,
}

/// Everything besides commands and bangs that the palette searches.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sources<'a> {
    pub tabs: &'a [TabEntry],
    pub current_tab: Option<usize>,
    /// History matches for the query, best first.
    pub history: &'a [TabEntry],
    /// Names of saved sessions.
    pub sessions: &'a [String],
    /// Settings that can be changed in one step.
    pub settings: &'a [SettingOption],
}

/// What kind of thing a result is; QML picks the icon from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Tab,
    History,
    Session,
    Command,
    Setting,
    Bang,
    /// Open the address that was typed.
    Open,
    /// Search the web for what was typed.
    Search,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Tab => "tab",
            Kind::History => "history",
            Kind::Session => "session",
            Kind::Command => "command",
            Kind::Setting => "setting",
            Kind::Bang => "bang",
            Kind::Open => "open",
            Kind::Search => "search",
        }
    }
}

/// What choosing a result does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Switch to the tab at this index.
    SwitchTab(usize),
    /// Load this URL.
    Open(String),
    /// Replace the open tabs with this saved session.
    OpenSession(String),
    /// Save the open tabs as a session with this name.
    SaveSession(String),
    /// Run the command with this id.
    Run(&'static str),
    /// Write these config keys.
    Set(Vec<(&'static str, SettingValue)>),
    /// Replace the palette's input with this text and keep it open.
    Complete(String),
}

/// One palette row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub kind: Kind,
    pub title: String,
    pub subtitle: String,
    /// Right-aligned hint: a shortcut or a `!trigger`.
    pub hint: String,
    pub action: Action,
    pub score: i32,
}

/// The palette's search over tabs, commands and bangs.
#[derive(Debug, Clone)]
pub struct Palette {
    omnibox: Omnibox,
    bangs: Arc<BangTable>,
}

impl Palette {
    /// `omnibox` should already include `bangs` as a step, as the URL bar's does.
    pub fn new(omnibox: Omnibox, bangs: Arc<BangTable>) -> Self {
        Self { omnibox, bangs }
    }

    /// Ranked results for `input`, best first.
    pub fn query(&self, input: &str, sources: &Sources) -> Vec<Item> {
        let Sources {
            tabs,
            current_tab,
            history,
            sessions,
            settings,
        } = *sources;
        let input = input.trim_start();

        if let Some(rest) = input.strip_prefix('>') {
            let mut items: Vec<Item> = save_session_item(rest.trim()).into_iter().collect();
            items.extend(commands(rest.trim()));
            items.extend(setting_items(settings, rest.trim()));
            return finish(items);
        }

        if let Some(trigger) = input.strip_prefix('!') {
            if !trigger.contains(char::is_whitespace) {
                return finish(self.bang_completions(trigger));
            }
        }

        let query = input.trim();
        if query.is_empty() {
            let mut items = tab_items(tabs, current_tab, "");
            items.extend(commands(""));
            return finish(items);
        }

        if let Some(item) = self.bang_search(query) {
            return vec![item];
        }

        let mut items: Vec<Item> = save_session_item(query).into_iter().collect();
        items.extend(tab_items(tabs, current_tab, query));
        items.extend(history_items(history, tabs, query));
        items.extend(session_items(sessions, query));
        items.extend(commands(query));
        items.extend(setting_items(settings, query));
        items.extend(self.bangs.iter().filter_map(|bang| {
            let score = fuzzy::score(query, &bang.name).max(fuzzy::score(query, &bang.trigger))?
                - BANG_PENALTY;
            Some(Item {
                kind: Kind::Bang,
                title: format!("Search {}", bang.name),
                subtitle: bang.home(),
                hint: format!("!{}", bang.trigger),
                action: Action::Complete(format!("!{} ", bang.trigger)),
                score,
            })
        }));
        if let Some(item) = self.typed(query) {
            items.push(item);
        }
        finish(items)
    }

    /// Suggestions under the URL bar for `input`, at most [`SUGGESTIONS`].
    ///
    /// The first row is always what Enter would do with the text as typed;
    /// after it come bang completions while a `!trigger` is being typed, then
    /// other open tabs and history pages. Commands, settings and sessions stay
    /// in the palette so the URL bar remains about places.
    pub fn suggest(&self, input: &str, sources: &Sources) -> Vec<Item> {
        let query = input.trim();
        let Some(first) = self.bang_search(query).or_else(|| self.typed(query)) else {
            return Vec::new();
        };

        let mut rest = Vec::new();
        // Typing a trigger without the "!" offers that site's search; Tab
        // (or choosing it) switches the input to "!trigger ".
        if let Some(bang) = self.bangs.get(query).filter(|_| !query.starts_with('!')) {
            rest.push(Item {
                kind: Kind::Bang,
                title: format!("Search {}", bang.name),
                subtitle: bang.home(),
                hint: "Tab".to_owned(),
                action: Action::Complete(format!("!{} ", bang.trigger)),
                score: i32::MAX,
            });
        }
        if let Some(trigger) = query.strip_prefix('!') {
            if !trigger.contains(char::is_whitespace) {
                let exact = trigger.to_lowercase();
                rest.extend(
                    self.bang_completions(trigger)
                        .into_iter()
                        .filter(|item| item.hint[1..] != exact),
                );
            }
        }
        if !query.starts_with('!') {
            // Switching to the tab you're already in isn't a suggestion.
            rest.extend(
                tab_items(sources.tabs, sources.current_tab, query)
                    .into_iter()
                    .filter(|item| {
                        !matches!(item.action, Action::SwitchTab(i) if Some(i) == sources.current_tab)
                    })
                    .map(|item| Item {
                        hint: "Switch to tab".to_owned(),
                        ..item
                    }),
            );
            rest.extend(history_items(sources.history, sources.tabs, query));
        }
        let mut items = vec![first];
        items.extend(finish(rest).into_iter().take(SUGGESTIONS - 1));
        items
    }

    /// The single result for input that uses a known bang, like `!gh ion`.
    fn bang_search(&self, query: &str) -> Option<Item> {
        let (bang, rest) = self.bangs.parse(query)?;
        let title = if rest.is_empty() {
            format!("Open {}", bang.name)
        } else {
            format!("Search {} for “{rest}”", bang.name)
        };
        let url = bang.url_for(rest);
        Some(Item {
            kind: Kind::Bang,
            title,
            subtitle: url.clone(),
            hint: format!("!{}", bang.trigger),
            action: Action::Open(url),
            score: i32::MAX,
        })
    }

    /// Bangs whose trigger or name matches what follows the `!`.
    fn bang_completions(&self, trigger: &str) -> Vec<Item> {
        self.bangs
            .iter()
            .filter_map(|bang| {
                let score = if trigger.is_empty() {
                    0
                } else if bang.trigger == trigger.to_lowercase() {
                    i32::MAX
                } else {
                    fuzzy::score(trigger, &bang.trigger)
                        .max(fuzzy::score(trigger, &bang.name).map(|s| s - BANG_PENALTY))?
                };
                Some(Item {
                    kind: Kind::Bang,
                    title: bang.name.clone(),
                    subtitle: bang.home(),
                    hint: format!("!{}", bang.trigger),
                    action: Action::Complete(format!("!{} ", bang.trigger)),
                    score,
                })
            })
            .collect()
    }

    /// The URL bar's reading of `query`: open it as an address or search for it.
    fn typed(&self, query: &str) -> Option<Item> {
        Some(match self.omnibox.resolve(query)? {
            Resolved::Url(url) => Item {
                kind: Kind::Open,
                title: format!("Open {url}"),
                subtitle: String::new(),
                hint: String::new(),
                action: Action::Open(url),
                score: i32::MAX,
            },
            Resolved::Search(url) | Resolved::Expanded(url) => Item {
                kind: Kind::Search,
                title: format!("Search {} for “{query}”", self.omnibox.search.name),
                subtitle: String::new(),
                hint: String::new(),
                action: Action::Open(url),
                // Ranks below substring matches and above scattered ones.
                score: 2 * query.chars().filter(|c| !c.is_whitespace()).count() as i32,
            },
        })
    }
}

fn tab_items(tabs: &[TabEntry], current: Option<usize>, query: &str) -> Vec<Item> {
    tabs.iter()
        .enumerate()
        .filter_map(|(index, tab)| {
            let score = fuzzy::score(query, &tab.title).max(fuzzy::score(query, &tab.url))?;
            let title = if tab.title.is_empty() {
                if tab.url.is_empty() {
                    "New tab"
                } else {
                    &tab.url
                }
            } else {
                &tab.title
            };
            Some(Item {
                kind: Kind::Tab,
                title: title.to_owned(),
                subtitle: tab.url.clone(),
                hint: if current == Some(index) {
                    "Current".to_owned()
                } else {
                    String::new()
                },
                action: Action::SwitchTab(index),
                score,
            })
        })
        .collect()
}

/// History pages that aren't already open in a tab.
fn history_items(history: &[TabEntry], tabs: &[TabEntry], query: &str) -> Vec<Item> {
    history
        .iter()
        .filter(|page| !tabs.iter().any(|tab| tab.url == page.url))
        .filter_map(|page| {
            let score = fuzzy::score(query, &page.title).max(fuzzy::score(query, &page.url))?
                - HISTORY_PENALTY;
            Some(Item {
                kind: Kind::History,
                title: if page.title.is_empty() {
                    page.url.clone()
                } else {
                    page.title.clone()
                },
                subtitle: page.url.clone(),
                hint: String::new(),
                action: Action::Open(page.url.clone()),
                score,
            })
        })
        .collect()
}

fn session_items(sessions: &[String], query: &str) -> Vec<Item> {
    sessions
        .iter()
        .filter_map(|name| {
            let score =
                fuzzy::score(query, name).max(fuzzy::score(query, &format!("{name} session")))?;
            Some(Item {
                kind: Kind::Session,
                title: name.clone(),
                subtitle: "Saved session".to_owned(),
                hint: String::new(),
                action: Action::OpenSession(name.clone()),
                score,
            })
        })
        .collect()
}

/// "save session work" (or "save as work") offers to save the open tabs as
/// the session "work".
fn save_session_item(query: &str) -> Option<Item> {
    let lower = query.to_lowercase();
    let prefix = ["save session as ", "save session ", "save as "]
        .into_iter()
        .find(|prefix| lower.starts_with(prefix))?;
    // The prefixes are ASCII, so lowercasing kept their byte length.
    let name = query[prefix.len()..].trim();
    if name.is_empty() {
        return None;
    }
    Some(Item {
        kind: Kind::Session,
        title: format!("Save open tabs as session “{name}”"),
        subtitle: String::new(),
        hint: String::new(),
        action: Action::SaveSession(name.to_owned()),
        score: i32::MAX,
    })
}

fn setting_items(settings: &[SettingOption], query: &str) -> Vec<Item> {
    settings
        .iter()
        .filter_map(|option| {
            let score = fuzzy::score(query, &option.title).max(fuzzy::score(
                query,
                &format!("{} {}", option.title, option.keywords),
            ))?;
            Some(Item {
                kind: Kind::Setting,
                title: option.title.clone(),
                subtitle: String::new(),
                hint: if option.active {
                    "Current".to_owned()
                } else {
                    String::new()
                },
                action: Action::Set(option.changes.clone()),
                // Settings sit just below commands that match as well.
                score: score - 1,
            })
        })
        .collect()
}

fn commands(query: &str) -> Vec<Item> {
    COMMANDS
        .iter()
        .filter_map(|command| {
            let Command {
                id,
                title,
                keywords,
                shortcut,
            } = *command;
            let score = fuzzy::score(query, title)
                .max(fuzzy::score(query, &format!("{title} {keywords}")))?;
            Some(Item {
                kind: Kind::Command,
                title: title.to_owned(),
                subtitle: String::new(),
                hint: shortcut.to_owned(),
                action: Action::Run(id),
                score,
            })
        })
        .collect()
}

/// Best first; equal scores keep source order (tabs, history, sessions,
/// commands, settings, bangs, typed).
fn finish(mut items: Vec<Item>) -> Vec<Item> {
    items.sort_by_key(|item| std::cmp::Reverse(item.score));
    items.truncate(MAX_RESULTS);
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> Palette {
        let bangs = Arc::new(BangTable::default());
        Palette::new(Omnibox::default().with_step(bangs.clone()), bangs)
    }

    fn tabs() -> Vec<TabEntry> {
        vec![
            TabEntry {
                title: "Mischa-dev/ion: a browser".into(),
                url: "https://github.com/Mischa-dev/ion".into(),
            },
            TabEntry {
                title: "Rust Programming Language".into(),
                url: "https://www.rust-lang.org/".into(),
            },
            TabEntry::default(),
        ]
    }

    fn history() -> Vec<TabEntry> {
        vec![
            TabEntry {
                title: "The Rust Book".into(),
                url: "https://doc.rust-lang.org/book/".into(),
            },
            // Already open as a tab, so not listed again.
            TabEntry {
                title: "Rust Programming Language".into(),
                url: "https://www.rust-lang.org/".into(),
            },
        ]
    }

    fn query(input: &str) -> Vec<Item> {
        let tabs = tabs();
        let history = history();
        let sessions = vec!["work".to_owned()];
        let settings = crate::settings::options(&ion_config::Config::default(), ["nord"]);
        palette().query(
            input,
            &Sources {
                tabs: &tabs,
                current_tab: Some(1),
                history: &history,
                sessions: &sessions,
                settings: &settings,
            },
        )
    }

    #[test]
    fn empty_query_lists_tabs_then_commands() {
        let items = query("");
        assert_eq!(items.len(), 3 + COMMANDS.len());
        assert_eq!(items[0].action, Action::SwitchTab(0));
        assert_eq!(items[1].hint, "Current");
        assert_eq!(items[2].title, "New tab");
        assert_eq!(items[3].action, Action::Run(COMMANDS[0].id));
    }

    #[test]
    fn matching_tab_ranks_above_web_search() {
        let items = query("rust");
        assert_eq!(items[0].action, Action::SwitchTab(1));
        let search = items.iter().position(|i| i.kind == Kind::Search).unwrap();
        assert!(search > 0);
    }

    #[test]
    fn history_follows_open_tabs_and_skips_them() {
        let items = query("rust");
        let tab = items.iter().position(|i| i.action == Action::SwitchTab(1));
        let page = items.iter().position(|i| i.kind == Kind::History);
        assert!(tab < page, "{items:#?}");
        let pages: Vec<_> = items.iter().filter(|i| i.kind == Kind::History).collect();
        assert_eq!(pages.len(), 1);
        assert_eq!(
            pages[0].action,
            Action::Open("https://doc.rust-lang.org/book/".into())
        );
    }

    #[test]
    fn sessions_are_found_by_name() {
        let items = query("work");
        assert_eq!(items[0].action, Action::OpenSession("work".into()));
        assert!(query("session").iter().any(|i| i.kind == Kind::Session));
        assert!(query("").iter().all(|i| i.kind != Kind::Session));
    }

    #[test]
    fn tabs_match_on_url() {
        let items = query("github ion");
        assert_eq!(items[0].action, Action::SwitchTab(0));
    }

    #[test]
    fn commands_match_on_title_and_keywords() {
        assert_eq!(query("reload")[0].action, Action::Run("reload"));
        assert_eq!(query("refresh")[0].action, Action::Run("reload"));
        assert_eq!(query("nt")[0].action, Action::Run("new-tab"));
    }

    #[test]
    fn angle_prefix_lists_only_commands_and_settings() {
        let items = query(">");
        assert!(items.len() > COMMANDS.len());
        assert!(
            items
                .iter()
                .all(|i| matches!(i.kind, Kind::Command | Kind::Setting))
        );
        assert_eq!(items[0].action, Action::Run(COMMANDS[0].id));
        let items = query("> quit");
        assert_eq!(items[0].action, Action::Run("quit"));
    }

    #[test]
    fn settings_are_found_by_title_and_keywords() {
        let items = query("nord");
        assert_eq!(items[0].title, "Theme: Nord");
        assert!(matches!(items[0].action, Action::Set(_)));
        let items = query("vertical");
        assert_eq!(items[0].title, "Vertical tabs");
        let current = query("comfortable");
        assert_eq!(current[0].hint, "Current");
        assert!(query("").iter().all(|i| i.kind != Kind::Setting));
    }

    #[test]
    fn addresses_open_directly_at_the_top() {
        let items = query("example.com");
        assert_eq!(items[0].kind, Kind::Open);
        assert_eq!(items[0].action, Action::Open("https://example.com/".into()));
    }

    #[test]
    fn unmatched_text_offers_a_search() {
        let items = query("qwzx");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, Kind::Search);
        assert_eq!(
            items[0].action,
            Action::Open("https://duckduckgo.com/?q=qwzx".into())
        );
    }

    #[test]
    fn bang_with_query_is_the_only_result() {
        let items = query("!gh cxx-qt");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Search GitHub for “cxx-qt”");
        assert_eq!(
            items[0].action,
            Action::Open("https://github.com/search?q=cxx-qt".into())
        );
        assert_eq!(query("cxx-qt !gh"), items);
    }

    #[test]
    fn known_bang_with_trailing_space_opens_front_page() {
        let items = query("!gh ");
        assert_eq!(items[0].title, "Open GitHub");
        assert_eq!(items[0].action, Action::Open("https://github.com/".into()));
    }

    #[test]
    fn typing_a_trigger_completes_bangs() {
        let items = query("!");
        assert_eq!(items.len(), BangTable::default().len());
        assert!(
            items
                .iter()
                .all(|i| matches!(i.action, Action::Complete(_)))
        );

        let items = query("!n");
        assert_eq!(items[0].hint, "!nix");
        assert!(items.iter().any(|i| i.hint == "!npm"));

        let items = query("!gh");
        assert_eq!(items[0].action, Action::Complete("!gh ".into()));
    }

    #[test]
    fn bangs_are_found_by_name() {
        let items = query("wikipedia");
        let bang = items.iter().find(|i| i.kind == Kind::Bang).unwrap();
        assert_eq!(bang.action, Action::Complete("!w ".into()));
    }

    #[test]
    fn unknown_bang_falls_back_to_search() {
        let items = query("!zzz thing");
        assert_eq!(items.last().unwrap().kind, Kind::Search);
    }

    #[test]
    fn results_are_capped() {
        let many: Vec<TabEntry> = (0..200)
            .map(|i| TabEntry {
                title: format!("tab {i}"),
                url: String::new(),
            })
            .collect();
        let sources = Sources {
            tabs: &many,
            ..Sources::default()
        };
        assert_eq!(palette().query("", &sources).len(), MAX_RESULTS);
    }

    fn suggest(input: &str) -> Vec<Item> {
        let tabs = tabs();
        let history = history();
        palette().suggest(
            input,
            &Sources {
                tabs: &tabs,
                current_tab: Some(0),
                history: &history,
                ..Sources::default()
            },
        )
    }

    #[test]
    fn suggestions_start_with_what_enter_does() {
        assert!(suggest("  ").is_empty());

        // "rust" is also a bang trigger, so its search comes right after.
        let items = suggest("rust");
        assert_eq!(items[0].kind, Kind::Search);
        assert_eq!(items[1].action, Action::Complete("!rust ".into()));
        assert_eq!(items[2].action, Action::SwitchTab(1));
        assert_eq!(items[2].hint, "Switch to tab");
        assert_eq!(items[3].kind, Kind::History);

        let items = suggest("example.com");
        assert_eq!(items[0].kind, Kind::Open);
    }

    #[test]
    fn suggestions_skip_the_current_tab_commands_and_settings() {
        let items = suggest("ion");
        assert!(items.iter().all(|i| i.action != Action::SwitchTab(0)));
        assert!(suggest("reload").iter().all(|i| i.kind != Kind::Command));
        assert!(suggest("nord").iter().all(|i| i.kind != Kind::Setting));
    }

    #[test]
    fn suggestions_expand_and_complete_bangs() {
        let items = suggest("!gh ion");
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].action,
            Action::Open("https://github.com/search?q=ion".into())
        );

        // "!g" alone opens Google; other bangs starting with g follow.
        let items = suggest("!g");
        assert_eq!(items[0].title, "Open Google");
        assert!(items[1..].iter().all(|i| i.hint != "!g"));
        assert!(
            items[1..]
                .iter()
                .any(|i| i.action == Action::Complete("!gh ".into()))
        );
        assert!(items.len() <= SUGGESTIONS);
    }

    #[test]
    fn typing_save_session_offers_to_save_one() {
        for input in [
            "save session Work stuff",
            "Save as Work stuff",
            ">save session as Work stuff",
        ] {
            let items = query(input);
            assert_eq!(
                items[0].action,
                Action::SaveSession("Work stuff".into()),
                "{input}"
            );
        }
        assert!(
            query("save session")
                .iter()
                .all(|i| !matches!(i.action, Action::SaveSession(_)))
        );
    }

    #[test]
    fn new_commands_are_found() {
        assert_eq!(query(">copy")[0].action, Action::Run("copy-url"));
        assert_eq!(query("ads")[0].action, Action::Run("toggle-adblock"));
        assert_eq!(query("full screen")[0].action, Action::Run("fullscreen"));
    }

    #[test]
    fn typing_a_trigger_offers_tab_to_search() {
        let items = suggest("gh");
        assert_eq!(items[1].action, Action::Complete("!gh ".into()));
        assert_eq!(items[1].hint, "Tab");
        assert!(suggest("ghx").iter().all(|i| i.hint != "Tab"));
    }
}
