//! `AddressBar` QML singleton: the dropdown under the address bar, from
//! `ion_omnibox`. Ranks open tabs, bookmarks, history, quick actions, inline
//! answers and the search engine's suggestions; fills in addresses as you
//! type; and builds the suggestion requests `UrlBar.qml` sends.

// `suggest` takes one argument per source; QML can't pass a struct.
#![allow(clippy::too_many_arguments)]

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
        type AddressBar = super::AddressBarRust;

        /// Rows for `input` in `PaletteSearch.query`'s format, best first
        /// (see `ion_omnibox::suggest`). `typed` is the part the person typed
        /// when an address was filled in after it, else "". `titles`/`urls`
        /// are the open tabs and
        /// `current` the active one; `history` is `History.search()`'s JSON,
        /// `bookmarks` `Bookmarks.search()`'s, `chosen` `History.chosen()`'s;
        /// `remote` the search engine's suggestions. Rows may also have the
        /// action `copy` (value: text for the clipboard).
        #[qinvokable]
        fn suggest(
            self: &AddressBar,
            input: &QString,
            typed: &QString,
            titles: &QStringList,
            urls: &QStringList,
            current: i32,
            history: &QString,
            bookmarks: &QString,
            chosen: &QString,
            remote: &QStringList,
        ) -> QVariant;

        /// `input` with an address filled in, e.g. "git" → "github.com/":
        /// from pages picked before after typing it (`chosen`), then
        /// `history`, then the open tabs' `urls`. Unchanged when nothing fits.
        #[qinvokable]
        fn autocomplete(
            self: &AddressBar,
            input: &QString,
            urls: &QStringList,
            history: &QString,
            chosen: &QString,
        ) -> QString;

        /// The search engine request for suggestions on `input`, or "" when
        /// none should be sent: suggestions are off, or the input is an
        /// address, a bang, a sum or addressed to an agent.
        #[qinvokable]
        #[cxx_name = "suggestUrl"]
        fn suggest_url(self: &AddressBar, input: &QString) -> QString;

        /// Whether to look for `pageUrl`'s own search (OpenSearch): true
        /// once per site that has none remembered yet.
        #[qinvokable]
        #[cxx_name = "wantsSiteSearch"]
        fn wants_site_search(self: &AddressBar, page_url: &QString) -> bool;

        /// Remember the search described by OpenSearch `xml`, found on
        /// `pageUrl`, as a bang for that site. Returns whether it was new.
        #[qinvokable]
        #[cxx_name = "addSiteSearch"]
        fn add_site_search(self: &AddressBar, page_url: &QString, xml: &QString) -> bool;

        /// The suggestions in a search engine's `body`, without `query`.
        #[qinvokable]
        #[cxx_name = "parseSuggestions"]
        fn parse_suggestions(self: &AddressBar, body: &QString, query: &QString) -> QStringList;
    }
}

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use cxx_qt_lib::{QList, QString, QStringList, QVariant};
use ion_bangs::Action;
use ion_core::engines;
use ion_core::navigation::Resolved;
use ion_omnibox::opensearch::{self, SiteEngine, SiteEngines};
use ion_omnibox::{Page, Sources};

use super::palette::{current_palette, entries, history_entries, rows, strings};

/// Search engine suggestions shown at most.
const REMOTE_LIMIT: usize = 6;
/// Longer input is a sentence; nobody needs it completed.
const MAX_SUGGEST_INPUT: usize = 120;

#[derive(Default)]
pub struct AddressBarRust;

/// Sites' own searches, loaded from `site-searches.json` on first use, and
/// the hosts already looked at this run.
struct SiteSearches {
    engines: SiteEngines,
    tried: HashSet<String>,
    path: Option<PathBuf>,
}

static SITE_SEARCHES: Mutex<Option<SiteSearches>> = Mutex::new(None);
/// Bumped whenever a site search is added, so bang tables get rebuilt.
static SITE_SEARCHES_REVISION: AtomicU64 = AtomicU64::new(0);

fn with_site_searches<T>(f: impl FnOnce(&mut SiteSearches) -> T) -> T {
    let mut guard = SITE_SEARCHES.lock().unwrap_or_else(|e| e.into_inner());
    let searches = guard.get_or_insert_with(|| {
        let path = ion_session::paths::data_dir().map(|d| d.join("site-searches.json"));
        let engines = path
            .as_deref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|json| SiteEngines::from_json(&json).ok())
            .unwrap_or_default();
        SiteSearches {
            engines,
            tried: HashSet::new(),
            path,
        }
    });
    f(searches)
}

impl SiteSearches {
    /// Saves the engines and has bang tables rebuilt.
    fn changed(&self) {
        SITE_SEARCHES_REVISION.fetch_add(1, Ordering::Relaxed);
        if let Some(path) = &self.path
            && let Err(err) =
                ion_session::paths::write_atomic(path, self.engines.to_json().as_bytes())
        {
            eprintln!("ion: could not save site searches: {err}");
        }
    }
}

/// Forgets the searches learned on `site` and its subdomains, or on every
/// site: they record where the person has been, so they go with history.
pub fn forget_site_searches(site: Option<&str>) {
    with_site_searches(|s| {
        let forgot = match site {
            Some(site) => s.engines.forget_site(site),
            None => {
                let had = !s.engines.is_empty();
                s.engines.clear();
                had
            }
        };
        s.tried.clear();
        if forgot {
            s.changed();
        }
    })
}

/// The sites' searches found so far and a revision that changes with them.
pub fn site_searches() -> (u64, Vec<SiteEngine>) {
    with_site_searches(|s| {
        (
            SITE_SEARCHES_REVISION.load(Ordering::Relaxed),
            s.engines.iter().cloned().collect(),
        )
    })
}

/// The host of an http(s) page, without `www.`.
fn page_host(page_url: &str) -> Option<String> {
    let url = url::Url::parse(page_url).ok()?;
    if !matches!(url.scheme(), "https" | "http") {
        return None;
    }
    Some(url.host_str()?.trim_start_matches("www.").to_lowercase())
}

/// `History.search()`'s or `History.chosen()`'s JSON as pages.
fn pages(json: &str, weight_key: &str) -> Vec<Page> {
    let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(json) else {
        return Vec::new();
    };
    list.iter()
        .map(|page| Page {
            title: page["title"].as_str().unwrap_or_default().to_owned(),
            url: page["url"].as_str().unwrap_or_default().to_owned(),
            frecency: page[weight_key].as_f64().unwrap_or_default(),
        })
        .collect()
}

impl qobject::AddressBar {
    fn suggest(
        &self,
        input: &QString,
        typed: &QString,
        titles: &QStringList,
        urls: &QStringList,
        current: i32,
        history: &QString,
        bookmarks: &QString,
        chosen: &QString,
        remote: &QStringList,
    ) -> QVariant {
        let tabs = entries(titles, urls);
        let history = pages(&history.to_string(), "frecency");
        let bookmarks = history_entries(&bookmarks.to_string());
        let chosen = pages(&chosen.to_string(), "boost");
        let remote = strings(remote);
        let typed = typed.to_string();
        let sources = Sources {
            tabs: &tabs,
            current_tab: usize::try_from(current).ok(),
            history: &history,
            bookmarks: &bookmarks,
            chosen: &chosen,
            remote: &remote,
            typed: &typed,
        };
        let mut items = ion_omnibox::suggest(&current_palette(), &input.to_string(), &sources);
        // Agent quick actions only once agents are turned on.
        if !ion_config::global().config().ai.enable {
            items.retain(|item| {
                !matches!(item.action, Action::Run(id) if ion_bangs::commands::AGENT_COMMANDS.contains(&id))
            });
        }
        // Quick actions show the keys in effect, after `[shortcuts]` remaps,
        // as the palette does.
        let keymap = super::keymap::current();
        for item in &mut items {
            if let Action::Run(id) = item.action
                && let Some(binding) = keymap.0.bindings().iter().find(|b| b.id == id)
            {
                item.hint = binding.keys.first().cloned().unwrap_or_default();
            }
        }
        rows(items)
    }

    fn autocomplete(
        &self,
        input: &QString,
        urls: &QStringList,
        history: &QString,
        chosen: &QString,
    ) -> QString {
        let input = input.to_string();
        let urls = strings(urls);
        let history = pages(&history.to_string(), "frecency");
        let chosen = pages(&chosen.to_string(), "boost");
        let candidates = chosen
            .iter()
            .chain(&history)
            .map(|page| page.url.as_str())
            .chain(urls.iter().map(String::as_str));
        QString::from(
            ion_bangs::autofill::complete(&input, candidates)
                .unwrap_or(input)
                .as_str(),
        )
    }

    fn suggest_url(&self, input: &QString) -> QString {
        let input = input.to_string();
        let query = input.trim();
        let config = ion_config::global().config();
        let palette = current_palette();
        let skip = !config.search.suggestions
            || query.chars().count() > MAX_SUGGEST_INPUT
            || query.starts_with(['!', '@', '>', '*'])
            || ion_omnibox::answer::answer(query).is_some()
            || !matches!(palette.omnibox().resolve(query), Some(Resolved::Search(_)));
        if skip {
            return QString::default();
        }
        let template = engines::suggest_template(
            &config.search.engine,
            &config.search.template,
            &config.search.suggest_template,
        );
        QString::from(
            engines::suggest_url(&template, query)
                .unwrap_or_default()
                .as_str(),
        )
    }

    fn wants_site_search(&self, page_url: &QString) -> bool {
        let Some(host) = page_host(&page_url.to_string()) else {
            return false;
        };
        with_site_searches(|s| !s.engines.knows(&host) && s.tried.insert(host))
    }

    fn add_site_search(&self, page_url: &QString, xml: &QString) -> bool {
        let Some(host) = page_host(&page_url.to_string()) else {
            return false;
        };
        let Some(engine) = opensearch::parse(&xml.to_string(), &host) else {
            return false;
        };
        with_site_searches(|s| {
            if !s.engines.add(engine) {
                return false;
            }
            s.changed();
            true
        })
    }

    fn parse_suggestions(&self, body: &QString, query: &QString) -> QStringList {
        let mut list = QList::<QString>::default();
        for suggestion in
            ion_omnibox::remote::parse(&body.to_string(), &query.to_string(), REMOTE_LIMIT)
        {
            list.append(QString::from(suggestion.as_str()));
        }
        QStringList::from(&list)
    }
}
