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

/// The lists Ion blocks with out of the box: EasyList for ads, EasyPrivacy for
/// trackers, and uBlock Origin's own lists, including its "unbreak" list of
/// exceptions that keeps the other lists from breaking sites.
pub fn defaults() -> Vec<FilterList> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ids_are_unique_file_names() {
        let lists = defaults();
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
    fn defaults_download_over_https() {
        assert!(defaults().iter().all(|l| l.url.starts_with("https://")));
    }
}
