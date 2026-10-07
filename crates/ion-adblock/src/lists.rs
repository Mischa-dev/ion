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

/// How old a cached list may get before it is downloaded again. EasyList and
/// the uBlock Origin lists all declare `Expires: 4 days` or shorter.
pub const MAX_AGE: Duration = Duration::from_secs(4 * 24 * 60 * 60);

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
    fn catalog_downloads_over_https() {
        assert!(catalog().iter().all(|l| l.url.starts_with("https://")));
    }
}
