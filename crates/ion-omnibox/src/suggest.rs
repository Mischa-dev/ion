//! Ranking the dropdown under the address bar.
//!
//! The rows, top to bottom:
//!
//! 1. An inline answer (`2+2`, `5 km to mi`), which Enter copies.
//! 2. What Enter does with the text as typed: open the address or search.
//! 3. Site search for a typed bang trigger (`gh`, then Tab), or bang
//!    completions while `!trigger` is being typed.
//! 4. Pages: open tabs (switch to them), bookmarks and history in one list,
//!    ranked by how well they match times how much they're used, plus what
//!    was picked after typing this before ([`crate::inputs`]).
//! 5. Quick actions: Ion commands whose name was typed (`clear history`).
//! 6. The search engine's suggestions.
//!
//! An empty address bar lists the most visited sites instead.
//!
//! Matching is by word starts and substrings, not scattered letters: in an
//! address bar `gh` should mean github.com, not every page with a g and an h.

use ion_bangs::{Action, COMMANDS, Item, Kind, Palette, TabEntry};
use ion_core::navigation::Resolved;

use crate::answer;

/// Most rows under the address bar.
pub const MAX_ROWS: usize = 10;
/// Most page rows (tabs, bookmarks, history).
const MAX_PAGES: usize = 6;
/// Most search engine suggestions, and the fewest kept when there are some.
const MAX_REMOTE: usize = 5;
const MIN_REMOTE: usize = 3;
/// Most quick actions.
const MAX_ACTIONS: usize = 2;
/// Sites listed while the address bar is empty.
pub const TOP_SITES: usize = 8;

/// A history page with how often and recently it was visited.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Page {
    pub title: String,
    pub url: String,
    /// `HistoryEntry::frecency`: visits weighted by recency.
    pub frecency: f64,
}

/// Everything the address bar ranks.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sources<'a> {
    /// Open tabs in order.
    pub tabs: &'a [TabEntry],
    pub current_tab: Option<usize>,
    /// History matches for the query, best first; for an empty query, the
    /// most frecent pages.
    pub history: &'a [Page],
    /// Bookmark matches for the query.
    pub bookmarks: &'a [TabEntry],
    /// Pages picked before after typing this (see [`crate::inputs`]), as
    /// pages whose `frecency` is the boost, strongest first.
    pub chosen: &'a [Page],
    /// The search engine's suggestions for the query, best first.
    pub remote: &'a [String],
    /// What the person typed when the input also holds an address filled in
    /// after it ("git" of "github.com/"). Pages are ranked for this; empty
    /// means the input was typed in full.
    pub typed: &'a str,
}

/// Rows for `input`, best first, at most [`MAX_ROWS`].
pub fn suggest(palette: &Palette, input: &str, sources: &Sources) -> Vec<Item> {
    let query = input.trim();
    if query.is_empty() {
        return top_sites(sources);
    }
    let bang = palette.bang_search(query);
    let typing_trigger = query
        .strip_prefix('!')
        .is_some_and(|trigger| !trigger.contains(char::is_whitespace));
    if let Some(item) = bang.clone().filter(|_| !typing_trigger) {
        return vec![item];
    }

    let mut rows = Vec::new();
    if let Some(answer) = answer::answer(query).filter(|_| bang.is_none()) {
        rows.push(Item {
            kind: Kind::Answer,
            title: answer.text,
            subtitle: String::new(),
            hint: "Copy".to_owned(),
            action: Action::Copy(answer.value),
            score: i32::MAX,
        });
    }
    rows.extend(bang.or_else(|| palette.typed(query)));

    if let Some(trigger) = query.strip_prefix('!') {
        if !trigger.contains(char::is_whitespace) {
            let exact = trigger.to_lowercase();
            let mut bangs = palette.bang_completions(trigger);
            bangs.retain(|item| item.hint[1..] != exact);
            bangs.sort_by_key(|item| std::cmp::Reverse(item.score));
            rows.extend(bangs);
        }
        rows.truncate(MAX_ROWS);
        return rows;
    }

    // Rank pages for what was typed, not for an address filled in after it.
    let typed = sources.typed.trim();
    let query = if !typed.is_empty() && query.to_lowercase().starts_with(&typed.to_lowercase()) {
        typed
    } else {
        query
    };

    // Typing a trigger without the "!" offers that site's search; Tab (or
    // choosing it) switches the input to "!trigger ".
    if let Some(bang) = palette.bangs().get(query) {
        rows.push(Item {
            kind: Kind::Bang,
            title: format!("Search {}", bang.name),
            subtitle: bang.home(),
            hint: "Tab".to_owned(),
            action: Action::Complete(format!("!{} ", bang.trigger)),
            score: i32::MAX,
        });
    }

    let mut pages = pages(query, sources);
    // The address Enter opens, when it's a known page, reads as that page.
    // An open tab keeps its own row, so it can still be switched to.
    if let Some(heading) = rows.iter_mut().find(|row| row.kind == Kind::Open)
        && let Action::Open(url) = &heading.action
        && let Some(at) = pages.iter().position(|page| {
            page.kind != Kind::Tab
                && normalize(&page.subtitle) == normalize(url)
                && page.title != page.subtitle
        })
    {
        let page = pages.remove(at);
        heading.title = page.title;
        heading.subtitle = page.subtitle;
    }
    let actions = quick_actions(query);
    let remote = remote_rows(palette, query, sources.remote, &pages);

    let reserved = rows.len() + actions.len() + remote.len().min(MIN_REMOTE);
    let page_rows = pages
        .len()
        .min(MAX_PAGES)
        .min(MAX_ROWS.saturating_sub(reserved));
    rows.extend(pages.into_iter().take(page_rows));
    rows.extend(actions);
    let room = MAX_ROWS.saturating_sub(rows.len());
    rows.extend(remote.into_iter().take(room.min(MAX_REMOTE)));
    rows.truncate(MAX_ROWS);
    rows
}

/// The most visited sites, for an address bar with nothing typed.
fn top_sites(sources: &Sources) -> Vec<Item> {
    let current = current_url(sources);
    let mut seen: Vec<String> = Vec::new();
    let mut rows = Vec::new();
    for page in sources.history {
        let key = normalize(&page.url);
        if Some(key.as_str()) == current.as_deref() || seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let tab = open_tab(sources, &page.url);
        rows.push(page_item(
            if tab.is_some() {
                Kind::Tab
            } else {
                Kind::History
            },
            &page.title,
            &page.url,
            tab,
            0,
        ));
        if rows.len() == TOP_SITES {
            break;
        }
    }
    rows
}

/// One page under consideration, merged across tabs, bookmarks and history.
struct Candidate {
    kind: Kind,
    title: String,
    url: String,
    tab: Option<usize>,
    /// How well the query matches, `None` until something matched.
    matched: Option<i32>,
    boost: f64,
}

/// Candidates collected from every source, one per page.
struct Pool {
    terms: Vec<String>,
    current: Option<String>,
    candidates: Vec<Candidate>,
}

impl Pool {
    fn find(&mut self, url: &str) -> Option<&mut Candidate> {
        let key = normalize(url);
        self.candidates
            .iter_mut()
            .find(|c| normalize(&c.url) == key)
    }

    fn add(&mut self, kind: Kind, title: &str, url: &str, tab: Option<usize>, boost: f64) {
        if url.is_empty() || self.current.as_deref() == Some(normalize(url).as_str()) {
            return;
        }
        let matched = match_score(&self.terms, title, url);
        match self.find(url) {
            Some(candidate) => {
                // An open tab beats a bookmark, which beats a history page.
                if rank(kind) < rank(candidate.kind) {
                    candidate.kind = kind;
                    candidate.url = url.to_owned();
                    candidate.tab = tab;
                }
                if candidate.title.is_empty() {
                    candidate.title = title.to_owned();
                }
                candidate.matched = candidate.matched.max(matched);
                candidate.boost += boost;
            }
            None => self.candidates.push(Candidate {
                kind,
                title: title.to_owned(),
                url: url.to_owned(),
                tab,
                matched,
                boost,
            }),
        }
    }
}

/// Open tabs, bookmarks and history pages matching `query`, best first.
fn pages(query: &str, sources: &Sources) -> Vec<Item> {
    let mut pool = Pool {
        // A typed or pasted address matches pages as they're compared:
        // without its scheme, `www.` or trailing slash.
        terms: query
            .split_whitespace()
            .map(|term| {
                if term.contains("://") || term.starts_with("www.") {
                    normalize(term)
                } else {
                    term.to_lowercase()
                }
            })
            .collect(),
        current: current_url(sources),
        candidates: Vec::new(),
    };
    for (index, tab) in sources.tabs.iter().enumerate() {
        if Some(index) != sources.current_tab {
            pool.add(Kind::Tab, &tab.title, &tab.url, Some(index), 5.0);
        }
    }
    for page in sources.bookmarks {
        pool.add(Kind::Bookmark, &page.title, &page.url, None, 6.0);
    }
    for page in sources.history {
        let usage = 4.0 * (1.0 + page.frecency.max(0.0)).ln();
        pool.add(Kind::History, &page.title, &page.url, None, usage);
    }
    for page in sources.chosen {
        // Learned choices count even when the text doesn't match the page.
        pool.add(Kind::History, &page.title, &page.url, None, 0.0);
        if let Some(candidate) = pool.find(&page.url) {
            candidate.boost += 12.0 * page.frecency.min(4.0);
            candidate.matched = candidate.matched.or(Some(6));
        }
    }

    let mut scored: Vec<(f64, Candidate)> = pool
        .candidates
        .into_iter()
        .filter_map(|c| Some((f64::from(c.matched?) + c.boost, c)))
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored
        .into_iter()
        .map(|(score, c)| page_item(c.kind, &c.title, &c.url, c.tab, score.round() as i32))
        .collect()
}

fn rank(kind: Kind) -> u8 {
    match kind {
        Kind::Tab => 0,
        Kind::Bookmark => 1,
        _ => 2,
    }
}

fn page_item(kind: Kind, title: &str, url: &str, tab: Option<usize>, score: i32) -> Item {
    Item {
        kind,
        title: if title.is_empty() {
            display_url(url).to_owned()
        } else {
            title.to_owned()
        },
        subtitle: display_url(url).to_owned(),
        hint: if tab.is_some() {
            "Switch to tab".to_owned()
        } else {
            String::new()
        },
        action: match tab {
            Some(index) => Action::SwitchTab(index),
            None => Action::Open(url.to_owned()),
        },
        score,
    }
}

fn current_url(sources: &Sources) -> Option<String> {
    let tab = sources.tabs.get(sources.current_tab?)?;
    (!tab.url.is_empty()).then(|| normalize(&tab.url))
}

fn open_tab(sources: &Sources, url: &str) -> Option<usize> {
    let key = normalize(url);
    sources
        .tabs
        .iter()
        .enumerate()
        .find(|(index, tab)| Some(*index) != sources.current_tab && normalize(&tab.url) == key)
        .map(|(index, _)| index)
}

/// `url` without `https://`, `www.` or a trailing `/`, for showing.
pub fn display_url(url: &str) -> &str {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    let trimmed = rest.strip_suffix('/').unwrap_or(rest);
    if trimmed.is_empty() { url } else { trimmed }
}

/// The same page whatever its scheme, `www.` or trailing slash.
fn normalize(url: &str) -> String {
    display_url(url).to_lowercase()
}

/// How well every term of the query matches a page, or `None` if one
/// doesn't. Site names count most, then word starts, then substrings.
fn match_score(terms: &[String], title: &str, url: &str) -> Option<i32> {
    if terms.is_empty() {
        return None;
    }
    let title = title.to_lowercase();
    let address = display_url(url).to_lowercase();
    let (host, path) = address.split_once('/').unwrap_or((&address, ""));
    let single_char = terms.len() == 1 && terms[0].chars().count() == 1;

    let mut total = 0;
    for (i, term) in terms.iter().enumerate() {
        let points = if host.starts_with(term.as_str()) {
            // The site itself: "git" → github.com.
            if i == 0 { 16 } else { 10 }
        } else if host
            .split(['.', '-'])
            .any(|part| part.starts_with(term.as_str()))
        {
            9
        } else if starts_word(&title, term) {
            7
        } else if single_char {
            return None;
        } else if starts_word(path, term) {
            6
        } else if term.chars().count() >= 3
            && (title.contains(term.as_str()) || path.contains(term.as_str()))
        {
            3
        } else {
            return None;
        };
        total += points;
    }
    // A short address for the site itself beats deep pages on it.
    if path.is_empty() && host.starts_with(terms[0].as_str()) {
        total += 4;
    }
    Some(total)
}

/// Whether `term` starts a word somewhere in `text` (both lowercase).
fn starts_word(text: &str, term: &str) -> bool {
    text.match_indices(term).any(|(at, _)| {
        text[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric())
    })
}

/// Ion commands named by what was typed: `clear hist`, `reader`, `screenshot`.
fn quick_actions(query: &str) -> Vec<Item> {
    if query.chars().count() < 3 {
        return Vec::new();
    }
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut actions: Vec<(i32, Item)> = COMMANDS
        .iter()
        .filter(|command| !command.hidden && command.id != "focus-url")
        .filter_map(|command| {
            let title = command.title.to_lowercase();
            let words = format!("{title} {}", command.keywords);
            // Every term starts a word of the name or its keywords, and at
            // least one is in the name itself.
            if !terms.iter().all(|t| starts_word(&words, t))
                || !terms.iter().any(|t| starts_word(&title, t))
            {
                return None;
            }
            let in_title = terms.iter().filter(|t| starts_word(&title, t)).count() as i32;
            let covered: usize = terms.iter().map(|t| t.chars().count()).sum();
            let score = in_title * 10 + (covered * 10 / title.chars().count().max(1)) as i32;
            Some((
                score,
                Item {
                    kind: Kind::Command,
                    title: command.title.to_owned(),
                    subtitle: String::new(),
                    hint: command.keys.first().copied().unwrap_or_default().to_owned(),
                    action: Action::Run(command.id),
                    score,
                },
            ))
        })
        .collect();
    actions.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    actions
        .into_iter()
        .map(|(_, item)| item)
        .take(MAX_ACTIONS)
        .collect()
}

/// The search engine's suggestions, minus any already shown as a page.
fn remote_rows(palette: &Palette, query: &str, remote: &[String], pages: &[Item]) -> Vec<Item> {
    let query = query.to_lowercase();
    remote
        .iter()
        .filter(|s| s.to_lowercase() != query)
        .filter_map(|suggestion| {
            let item = match palette.omnibox().resolve(suggestion)? {
                Resolved::Url(url) => {
                    if pages.iter().any(|p| p.action == Action::Open(url.clone())) {
                        return None;
                    }
                    Item {
                        kind: Kind::Open,
                        title: suggestion.clone(),
                        subtitle: String::new(),
                        hint: String::new(),
                        action: Action::Open(url),
                        score: 0,
                    }
                }
                Resolved::Search(url) | Resolved::Expanded(url) => Item {
                    kind: Kind::Suggestion,
                    title: suggestion.clone(),
                    subtitle: String::new(),
                    hint: String::new(),
                    action: Action::Open(url),
                    score: 0,
                },
            };
            Some(item)
        })
        .take(MAX_REMOTE)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ion_bangs::BangTable;
    use ion_core::navigation::Omnibox;

    use super::*;

    fn palette() -> Palette {
        let bangs = Arc::new(BangTable::default());
        Palette::new(Omnibox::default().with_step(bangs.clone()), bangs)
    }

    fn tab(title: &str, url: &str) -> TabEntry {
        TabEntry {
            title: title.into(),
            url: url.into(),
        }
    }

    fn page(title: &str, url: &str, frecency: f64) -> Page {
        Page {
            title: title.into(),
            url: url.into(),
            frecency,
        }
    }

    struct World {
        tabs: Vec<TabEntry>,
        history: Vec<Page>,
        bookmarks: Vec<TabEntry>,
        chosen: Vec<Page>,
        remote: Vec<String>,
    }

    fn world() -> World {
        World {
            tabs: vec![
                tab(
                    "Mischa-dev/ion: a browser",
                    "https://github.com/Mischa-dev/ion",
                ),
                tab("Rust Programming Language", "https://www.rust-lang.org/"),
            ],
            history: vec![
                page("GitHub", "https://github.com/", 40.0),
                page("The Rust Book", "https://doc.rust-lang.org/book/", 3.0),
                page(
                    "Rust Programming Language",
                    "https://www.rust-lang.org/",
                    9.0,
                ),
                page("Gitea docs", "https://docs.gitea.com/", 1.0),
                page("Hacker News", "https://news.ycombinator.com/", 25.0),
            ],
            bookmarks: vec![tab("Nix manual", "https://nixos.org/manual/")],
            chosen: Vec::new(),
            remote: vec!["rust book".into(), "rust analyzer".into(), "rust".into()],
        }
    }

    fn run(world: &World, input: &str) -> Vec<Item> {
        suggest(
            &palette(),
            input,
            &Sources {
                tabs: &world.tabs,
                current_tab: Some(0),
                history: &world.history,
                bookmarks: &world.bookmarks,
                chosen: &world.chosen,
                remote: &world.remote,
                typed: "",
            },
        )
    }

    fn titles(items: &[Item]) -> Vec<&str> {
        items.iter().map(|i| i.title.as_str()).collect()
    }

    #[test]
    fn first_row_is_what_enter_does() {
        let w = world();
        assert_eq!(run(&w, "qwzx")[0].kind, Kind::Search);
        assert_eq!(run(&w, "example.com")[0].kind, Kind::Open);
    }

    #[test]
    fn answers_come_first_and_copy() {
        let w = world();
        let items = run(&w, "2^10");
        assert_eq!(items[0].kind, Kind::Answer);
        assert_eq!(items[0].title, "= 1024");
        assert_eq!(items[0].action, Action::Copy("1024".into()));
        assert_eq!(items[1].kind, Kind::Search);
        assert_eq!(run(&w, "5 km to mi")[0].title, "5 km = 3.10686 mi");
    }

    #[test]
    fn sites_rank_by_name_and_use() {
        let w = world();
        let items = run(&w, "git");
        // github.com is visited far more than docs.gitea.com.
        assert_eq!(items[1].action, Action::Open("https://github.com/".into()));
        assert_eq!(items[1].subtitle, "github.com");
        let gitea = items.iter().position(|i| i.title == "Gitea docs").unwrap();
        assert!(gitea > 1);
        // "news" finds news.ycombinator.com by its host.
        assert_eq!(run(&w, "news")[1].title, "Hacker News");
        assert_eq!(run(&w, "ycomb")[1].title, "Hacker News");
    }

    #[test]
    fn open_tabs_switch_and_merge_with_history() {
        let w = world();
        let items = run(&w, "rust");
        let tab = items
            .iter()
            .find(|i| i.action == Action::SwitchTab(1))
            .unwrap();
        assert_eq!(tab.hint, "Switch to tab");
        // The same page isn't listed again from history.
        assert_eq!(
            items
                .iter()
                .filter(|i| i.title == "Rust Programming Language")
                .count(),
            1
        );
        // Typing or pasting an open tab's address still offers to switch to it.
        for address in ["rust-lang.org", "https://www.rust-lang.org/"] {
            assert!(
                run(&w, address)
                    .iter()
                    .any(|i| i.action == Action::SwitchTab(1)),
                "{address}"
            );
        }
        // The current tab isn't suggested.
        assert!(
            run(&w, "ion")
                .iter()
                .all(|i| i.action != Action::SwitchTab(0))
        );
    }

    #[test]
    fn scattered_letters_dont_match() {
        let w = world();
        // "gh" is a bang trigger; it must not drag in every page with g…h.
        let items = run(&w, "gh");
        assert!(
            items.iter().all(|i| i.kind != Kind::History),
            "{:?}",
            titles(&items)
        );
        assert_eq!(items[1].hint, "Tab");
        assert!(run(&w, "rbk").iter().all(|i| i.kind != Kind::History));
    }

    #[test]
    fn learned_choices_win_even_without_a_match() {
        let mut w = world();
        w.chosen = vec![page("Hacker News", "https://news.ycombinator.com/", 3.0)]; // picked after typing "orange site";
        let items = run(&w, "orange");
        assert_eq!(items[1].title, "Hacker News");
    }

    #[test]
    fn bookmarks_are_suggested() {
        let w = world();
        let items = run(&w, "nix man");
        assert_eq!(items[1].kind, Kind::Bookmark);
    }

    #[test]
    fn search_suggestions_follow_pages() {
        let w = world();
        let items = run(&w, "rust");
        let kinds: Vec<Kind> = items.iter().map(|i| i.kind).collect();
        let first_remote = kinds.iter().position(|k| *k == Kind::Suggestion).unwrap();
        assert!(kinds[..first_remote].contains(&Kind::History));
        let remote: Vec<&str> = items
            .iter()
            .filter(|i| i.kind == Kind::Suggestion)
            .map(|i| i.title.as_str())
            .collect();
        // The suggestion that only repeats the query is left out.
        assert_eq!(remote, ["rust book", "rust analyzer"]);
        assert_eq!(
            items[first_remote].action,
            Action::Open("https://duckduckgo.com/?q=rust+book".into())
        );
        assert!(items.len() <= MAX_ROWS);
    }

    #[test]
    fn quick_actions_show_for_command_names() {
        let w = world();
        let items = run(&w, "clear hist");
        let action = items.iter().find(|i| i.kind == Kind::Command).unwrap();
        assert_eq!(action.action, Action::Run("clear-history"));
        assert!(run(&w, "re").iter().all(|i| i.kind != Kind::Command));
        assert!(run(&w, "rust").iter().all(|i| i.kind != Kind::Command));
    }

    #[test]
    fn empty_input_lists_top_sites() {
        let w = world();
        let items = run(&w, "");
        assert_eq!(items[0].title, "GitHub");
        // Open elsewhere: switch to it rather than load it twice.
        let rust = items
            .iter()
            .find(|i| i.title == "Rust Programming Language")
            .unwrap();
        assert_eq!(rust.action, Action::SwitchTab(1));
        assert!(items.len() <= TOP_SITES);
    }

    #[test]
    fn bangs_expand_and_complete() {
        let w = world();
        let items = run(&w, "!gh ion");
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].action,
            Action::Open("https://github.com/search?q=ion".into())
        );
        let items = run(&w, "!g");
        assert_eq!(items[0].title, "Open Google");
        assert!(items[1..].iter().all(|i| i.hint != "!g"));
        assert!(
            items[1..]
                .iter()
                .any(|i| i.action == Action::Complete("!gh ".into()))
        );
        assert!(items.len() <= MAX_ROWS);
    }

    #[test]
    fn filled_in_addresses_rank_for_what_was_typed() {
        let w = world();
        let items = suggest(
            &palette(),
            "github.com/",
            &Sources {
                tabs: &w.tabs,
                current_tab: Some(1),
                history: &w.history,
                typed: "git",
                ..Sources::default()
            },
        );
        // Enter opens github.com, shown as the page it is.
        assert_eq!(items[0].kind, Kind::Open);
        assert_eq!(items[0].title, "GitHub");
        assert_eq!(items[0].subtitle, "github.com");
        assert_eq!(items[0].action, Action::Open("https://github.com/".into()));
        // The rest is ranked for "git": the open ion tab and Gitea follow.
        assert!(items.iter().any(|i| i.action == Action::SwitchTab(0)));
        assert!(items.iter().any(|i| i.title == "Gitea docs"));
        assert_eq!(items.iter().filter(|i| i.title == "GitHub").count(), 1);
    }

    #[test]
    fn display_urls_drop_the_noise() {
        assert_eq!(display_url("https://www.github.com/"), "github.com");
        assert_eq!(display_url("http://a.example/b/"), "a.example/b");
        assert_eq!(display_url("file:///tmp/x.html"), "file:///tmp/x.html");
        assert_eq!(display_url("https://"), "https://");
    }
}
