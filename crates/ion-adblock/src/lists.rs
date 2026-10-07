//! The filter lists Ion downloads by default.

use std::time::Duration;

/// A filter list in Adblock Plus / uBlock Origin syntax.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterList {
    /// Stable identifier, used as the cache file name.
    pub id: String,
    /// Human-facing name.
    pub title: String,
    /// Where the list is downloaded from.
    pub url: String,
}

impl FilterList {
    pub fn new(id: impl Into<String>, title: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            url: url.into(),
        }
    }
}

/// How old a cached list may get before it is downloaded again, unless the
/// list declares its own `! Expires:` (EasyList: 4 days; uBlock's quick
/// fixes: 8 hours).
pub const MAX_AGE: Duration = Duration::from_secs(4 * 24 * 60 * 60);

const HOUR: u64 = 60 * 60;

/// The list's own `! Expires: 4 days (update frequency)` header, kept between
/// an hour and two weeks; [`MAX_AGE`] when it has none.
pub fn expiry_of(text: &str) -> Duration {
    let declared = text
        .lines()
        .take(50)
        .take_while(|line| line.trim().is_empty() || line.starts_with(['!', '[', '#']))
        .find_map(|line| {
            let rest = line.trim_start_matches(['!', '#']).trim();
            let value = rest.strip_prefix("Expires:")?.trim();
            let mut words = value.split_whitespace();
            let n: u64 = words.next()?.parse().ok()?;
            let unit = words.next().unwrap_or("days");
            let hours = if unit.starts_with("hour") {
                n
            } else if unit.starts_with("day") {
                n * 24
            } else {
                return None;
            };
            Some(Duration::from_secs(hours * HOUR))
        });
    declared.map_or(MAX_AGE, |d| {
        d.clamp(
            Duration::from_secs(HOUR),
            Duration::from_secs(14 * 24 * HOUR),
        )
    })
}

/// The lists Ion knows by name: EasyList for ads, EasyPrivacy for trackers,
/// and uBlock Origin's own lists, including its "unbreak" list of exceptions
/// that keeps the other lists from breaking sites.
pub fn catalog() -> Vec<FilterList> {
    const UBO: &str = "https://ublockorigin.github.io/uAssets/filters";
    vec![
        FilterList::new(
            "easylist",
            "EasyList",
            "https://easylist.to/easylist/easylist.txt",
        ),
        FilterList::new(
            "easyprivacy",
            "EasyPrivacy",
            "https://easylist.to/easylist/easyprivacy.txt",
        ),
        FilterList::new(
            "ublock-filters",
            "uBlock filters",
            format!("{UBO}/filters.txt"),
        ),
        FilterList::new(
            "ublock-privacy",
            "uBlock filters – Privacy",
            format!("{UBO}/privacy.txt"),
        ),
        FilterList::new(
            "ublock-unbreak",
            "uBlock filters – Unbreak",
            format!("{UBO}/unbreak.txt"),
        ),
        FilterList::new(
            "ublock-quick-fixes",
            "uBlock filters – Quick fixes",
            format!("{UBO}/quick-fixes.txt"),
        ),
    ]
}

/// The lists the `[adblock] lists` setting names, in order and without
/// duplicates: names from [`catalog`] or `https://` URLs of other lists.
/// `ublock-filters` brings uBlock Origin's unbreak and quick-fixes lists
/// along, as uBlock Origin itself does, since its filters assume them.
/// Returns the lists and the entries that were neither.
pub fn resolve(names: &[String]) -> (Vec<FilterList>, Vec<String>) {
    let catalog = catalog();
    let by_id = |id: &str| catalog.iter().find(|l| l.id == id).cloned();
    let mut lists: Vec<FilterList> = Vec::new();
    let mut unknown = Vec::new();
    let mut push = |list: FilterList| {
        if !lists.iter().any(|l| l.id == list.id) {
            lists.push(list);
        }
    };
    for name in names {
        let name = name.trim();
        if let Some(list) = by_id(name) {
            push(list);
            if name == "ublock-filters" {
                push(by_id("ublock-unbreak").expect("in catalog"));
                push(by_id("ublock-quick-fixes").expect("in catalog"));
            }
        } else if name.starts_with("https://") {
            push(FilterList::new(url_id(name), name, name));
        } else {
            unknown.push(name.to_owned());
        }
    }
    (lists, unknown)
}

/// A stable cache file name for a list given by URL.
fn url_id(url: &str) -> String {
    // FNV-1a: stable across Rust versions, unlike `DefaultHasher`.
    let hash = url.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("url-{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    fn ids(lists: &[FilterList]) -> Vec<&str> {
        lists.iter().map(|l| l.id.as_str()).collect()
    }

    #[test]
    fn resolves_names_and_urls() {
        let (lists, unknown) = resolve(&names(&[
            "easylist",
            "https://example.test/list.txt",
            "nope",
            "easylist",
        ]));
        assert_eq!(ids(&lists)[0], "easylist");
        assert_eq!(lists.len(), 2);
        assert!(lists[1].id.starts_with("url-"));
        assert_eq!(lists[1].url, "https://example.test/list.txt");
        assert_eq!(unknown, ["nope"]);
    }

    #[test]
    fn ublock_filters_bring_their_companions() {
        let (lists, _) = resolve(&names(&["ublock-filters", "ublock-unbreak"]));
        assert_eq!(
            ids(&lists),
            ["ublock-filters", "ublock-unbreak", "ublock-quick-fixes"]
        );
    }

    #[test]
    fn url_ids_are_stable_file_names() {
        assert_eq!(url_id("https://a.test/x"), url_id("https://a.test/x"));
        assert_ne!(url_id("https://a.test/x"), url_id("https://a.test/y"));
        assert_eq!(url_id("https://a.test/x").len(), "url-".len() + 16);
    }

    #[test]
    fn catalog_ids_are_unique_file_names() {
        let lists = catalog();
        let mut ids: Vec<_> = lists.iter().map(|l| l.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), lists.len());
        for id in ids {
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{id}"
            );
        }
    }

    #[test]
    fn reads_declared_expiry() {
        let hours = |h: u64| Duration::from_secs(h * HOUR);
        assert_eq!(
            expiry_of("! Title: Quick fixes\n! Expires: 8 hours\n||a.test^\n"),
            hours(8)
        );
        assert_eq!(
            expiry_of("[Adblock Plus 2.0]\n! Expires: 1 days (update frequency)\n"),
            hours(24)
        );
        assert_eq!(expiry_of("! Expires: 0 hours\n"), hours(1));
        assert_eq!(expiry_of("! Expires: 90 days\n"), hours(14 * 24));
        assert_eq!(expiry_of("! Title: none\n||a.test^\n"), MAX_AGE);
        // Only the header counts, not a comment deep in the rules.
        assert_eq!(expiry_of("||a.test^\n! Expires: 1 hours\n"), MAX_AGE);
    }

    #[test]
    fn catalog_downloads_over_https() {
        assert!(catalog().iter().all(|l| l.url.starts_with("https://")));
    }
}
