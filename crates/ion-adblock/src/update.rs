//! Keeping the filter lists fresh and turning them into a [`Blocker`].
//!
//! Runs on a worker thread: downloading and compiling the default lists takes
//! a second or two, which must never block the UI.

use std::time::{Duration, SystemTime};

use crate::lists::MAX_AGE;
use crate::{Blocker, FilterList, Store};

/// Fetches a list's text. [`HttpFetch`] in the app, a fake in tests.
pub trait Fetch {
    fn fetch(&self, url: &str) -> Result<String, String>;
}

/// Downloads lists over HTTPS. Honours the usual `HTTPS_PROXY` variables.
pub struct HttpFetch {
    agent: ureq::Agent,
}

/// Lists are a few MB at most; anything far bigger is not a filter list.
const MAX_LIST_BYTES: u64 = 32 * 1024 * 1024;

impl Default for HttpFetch {
    fn default() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(60)))
            .user_agent(concat!("Ion/", env!("CARGO_PKG_VERSION")))
            .build();
        Self {
            agent: config.into(),
        }
    }
}

impl Fetch for HttpFetch {
    fn fetch(&self, url: &str) -> Result<String, String> {
        self.agent
            .get(url)
            .call()
            .map_err(|e| e.to_string())?
            .body_mut()
            .with_config()
            .limit(MAX_LIST_BYTES)
            .read_to_string()
            .map_err(|e| e.to_string())
    }
}

/// What a [`refresh`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Ids of the lists that were downloaded and saved.
    pub downloaded: Vec<String>,
    /// Ids of the lists that could not be updated, with the reason.
    pub failed: Vec<(String, String)>,
}

impl Report {
    pub fn changed(&self) -> bool {
        !self.downloaded.is_empty()
    }
}

/// Whether `list` is missing from the cache or older than [`MAX_AGE`].
pub fn is_stale(store: &Store, list: &FilterList, now: SystemTime) -> bool {
    store
        .list_age(&list.id, now)
        .is_none_or(|age| age > MAX_AGE)
}

/// Download the lists that are stale (or all of them with `force`) and save
/// them. A failed download keeps the previously cached copy.
pub fn refresh(
    store: &Store,
    lists: &[FilterList],
    fetch: &dyn Fetch,
    now: SystemTime,
    force: bool,
) -> Report {
    let mut report = Report::default();
    for list in lists {
        if !force && !is_stale(store, list, now) {
            continue;
        }
        let result = fetch
            .fetch(&list.url)
            .and_then(|text| {
                if looks_like_filter_list(&text) {
                    Ok(text)
                } else {
                    Err("response is not a filter list".to_owned())
                }
            })
            .and_then(|text| store.write_list(&list.id, &text).map_err(|e| e.to_string()));
        match result {
            Ok(()) => report.downloaded.push(list.id.clone()),
            Err(e) => report.failed.push((list.id.clone(), e)),
        }
    }
    report
}

/// Compile the cached lists, save the result for the next start, and return
/// it. `None` when no list is cached yet.
pub fn build(store: &Store, lists: &[FilterList]) -> Option<Blocker> {
    let texts: Vec<String> = lists
        .iter()
        .filter_map(|l| store.read_list(&l.id))
        .collect();
    if texts.is_empty() {
        return None;
    }
    let blocker = Blocker::from_lists(texts.iter().map(String::as_str));
    // Not fatal: the next start just compiles again.
    let _ = store.write_engine(lists, &blocker);
    Some(blocker)
}

/// The fastest way to a working blocker from the cache: the compiled engine
/// if it is current, else compile the cached lists.
pub fn load(store: &Store, lists: &[FilterList]) -> Option<Blocker> {
    store.read_engine(lists).or_else(|| build(store, lists))
}

/// Guards against saving a captive portal page or an error page as a list.
fn looks_like_filter_list(text: &str) -> bool {
    let start = text.trim_start();
    !start.starts_with('<')
        && start
            .lines()
            .any(|line| !line.trim().is_empty() && !line.starts_with('!') && !line.starts_with('['))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use super::*;
    use crate::store::tests::temp_dir;
    use crate::{RequestInfo, ResourceType};

    #[derive(Default)]
    struct FakeFetch {
        responses: HashMap<String, Result<String, String>>,
        calls: RefCell<Vec<String>>,
    }

    impl FakeFetch {
        fn with(mut self, url: &str, response: Result<&str, &str>) -> Self {
            self.responses.insert(
                url.to_owned(),
                response.map(str::to_owned).map_err(str::to_owned),
            );
            self
        }
    }

    impl Fetch for FakeFetch {
        fn fetch(&self, url: &str) -> Result<String, String> {
            self.calls.borrow_mut().push(url.to_owned());
            self.responses
                .get(url)
                .cloned()
                .unwrap_or_else(|| Err("not found".to_owned()))
        }
    }

    fn lists() -> Vec<FilterList> {
        vec![
            FilterList::new("ads", "Ads", "https://lists.test/ads.txt"),
            FilterList::new("trackers", "Trackers", "https://lists.test/trackers.txt"),
        ]
    }

    fn fetch() -> FakeFetch {
        FakeFetch::default()
            .with(
                "https://lists.test/ads.txt",
                Ok("! Title: ads\n||ads.test^\n"),
            )
            .with("https://lists.test/trackers.txt", Ok("||tracker.test^\n"))
    }

    fn blocks(blocker: &Blocker, url: &str) -> bool {
        blocker.should_block(&RequestInfo::new(
            url,
            "https://site.test/",
            ResourceType::Script,
        ))
    }

    #[test]
    fn first_refresh_downloads_everything() {
        let store = Store::new(temp_dir("first"));
        let report = refresh(&store, &lists(), &fetch(), SystemTime::now(), false);
        assert_eq!(report.downloaded, ["ads", "trackers"]);
        assert!(report.failed.is_empty());
        assert!(report.changed());
    }

    #[test]
    fn fresh_lists_are_not_downloaded_again() {
        let store = Store::new(temp_dir("fresh"));
        let now = SystemTime::now();
        refresh(&store, &lists(), &fetch(), now, false);

        let again = fetch();
        let report = refresh(&store, &lists(), &again, now, false);
        assert_eq!(report, Report::default());
        assert!(again.calls.borrow().is_empty());

        let later = now + MAX_AGE + Duration::from_secs(60);
        let report = refresh(&store, &lists(), &again, later, false);
        assert_eq!(report.downloaded.len(), 2);

        let forced = fetch();
        refresh(&store, &lists(), &forced, now, true);
        assert_eq!(forced.calls.borrow().len(), 2);
    }

    #[test]
    fn failures_keep_the_cached_copy() {
        let store = Store::new(temp_dir("fail"));
        refresh(&store, &lists(), &fetch(), SystemTime::now(), false);

        let broken = FakeFetch::default()
            .with("https://lists.test/ads.txt", Err("offline"))
            .with(
                "https://lists.test/trackers.txt",
                Ok("<html>Sign in to Wi-Fi</html>"),
            );
        let report = refresh(&store, &lists(), &broken, SystemTime::now(), true);
        assert!(report.downloaded.is_empty());
        assert_eq!(report.failed.len(), 2);
        assert_eq!(
            store.read_list("trackers").as_deref(),
            Some("||tracker.test^\n")
        );
    }

    #[test]
    fn build_and_load() {
        let store = Store::new(temp_dir("build"));
        assert!(build(&store, &lists()).is_none());
        assert!(load(&store, &lists()).is_none());

        refresh(&store, &lists(), &fetch(), SystemTime::now(), false);
        let built = build(&store, &lists()).unwrap();
        assert!(blocks(&built, "https://ads.test/a.js"));
        assert!(blocks(&built, "https://tracker.test/t.js"));
        assert!(!blocks(&built, "https://site.test/app.js"));

        // `build` saved the engine, so `load` can skip compiling.
        assert!(store.read_engine(&lists()).is_some());
        let loaded = load(&store, &lists()).unwrap();
        assert!(blocks(&loaded, "https://ads.test/a.js"));
    }

    #[test]
    fn filter_list_sniffing() {
        assert!(looks_like_filter_list(
            "[Adblock Plus 2.0]\n! c\n||a.test^\n"
        ));
        assert!(!looks_like_filter_list("  <!DOCTYPE html><html>"));
        assert!(!looks_like_filter_list("! only comments\n\n"));
        assert!(!looks_like_filter_list(""));
    }
}
