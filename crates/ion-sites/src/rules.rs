//! Matching `[sites]` keys against URLs.
//!
//! A key is a host: `"example.com"` covers `example.com` and every subdomain,
//! `"*"` covers every site. When several keys match, the most specific (the
//! longest host) wins for single values.

use std::collections::BTreeMap;

use ion_config::Site;

/// The lowercase host of `url`, or `None` for URLs without one (`about:`,
/// `data:`, `file:`).
pub fn host_of(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    parsed
        .host_str()
        .filter(|h| !h.is_empty())
        .map(|h| h.trim_end_matches('.').to_ascii_lowercase())
}

/// Whether the `[sites]` key `key` covers `host`.
pub fn key_covers(key: &str, host: &str) -> bool {
    let key = key.trim().trim_end_matches('.').to_ascii_lowercase();
    if key == "*" {
        return true;
    }
    let key = key.strip_prefix("*.").unwrap_or(&key);
    host == key
        || host
            .strip_suffix(key)
            .is_some_and(|rest| rest.ends_with('.'))
}

/// The `[sites]` entries covering `url`, least specific first.
pub fn matching<'a>(sites: &'a BTreeMap<String, Site>, url: &str) -> Vec<(&'a str, &'a Site)> {
    let host = host_of(url);
    let mut hits: Vec<(&str, &Site)> = sites
        .iter()
        .filter(|(key, _)| match &host {
            Some(host) => key_covers(key, host),
            None => key.trim() == "*",
        })
        .map(|(k, v)| (k.as_str(), v))
        .collect();
    hits.sort_by_key(|(key, _)| specificity(key));
    hits
}

fn specificity(key: &str) -> usize {
    if key.trim() == "*" { 0 } else { key.len() + 1 }
}

/// Whether pages at `url` may run JavaScript. On unless the most specific
/// matching entry that sets `javascript` turns it off.
pub fn javascript_enabled(sites: &BTreeMap<String, Site>, url: &str) -> bool {
    matching(sites, url)
        .iter()
        .rev()
        .find_map(|(_, site)| site.javascript)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sites(entries: &[(&str, Option<bool>)]) -> BTreeMap<String, Site> {
        entries
            .iter()
            .map(|(k, js)| {
                (
                    (*k).to_owned(),
                    Site {
                        javascript: *js,
                        ..Site::default()
                    },
                )
            })
            .collect()
    }

    #[test]
    fn hosts_cover_their_subdomains_only() {
        assert!(key_covers("example.com", "example.com"));
        assert!(key_covers("example.com", "www.example.com"));
        assert!(key_covers("*.example.com", "a.b.example.com"));
        assert!(key_covers("Example.COM.", "example.com"));
        assert!(!key_covers("example.com", "notexample.com"));
        assert!(!key_covers("www.example.com", "example.com"));
        assert!(key_covers("*", "anything.org"));
    }

    #[test]
    fn host_of_handles_odd_urls() {
        assert_eq!(
            host_of("https://WWW.Example.com:8443/x?y").as_deref(),
            Some("www.example.com")
        );
        assert_eq!(host_of("about:blank"), None);
        assert_eq!(host_of("file:///tmp/a.html"), None);
        assert_eq!(host_of("not a url"), None);
    }

    #[test]
    fn most_specific_javascript_setting_wins() {
        let s = sites(&[
            ("*", Some(false)),
            ("example.com", Some(true)),
            ("ads.example.com", None),
        ]);
        assert!(javascript_enabled(&s, "https://ads.example.com/"));
        assert!(javascript_enabled(&s, "https://example.com/"));
        assert!(!javascript_enabled(&s, "https://other.org/"));
        assert!(!javascript_enabled(&s, "about:blank"));
        assert!(javascript_enabled(&BTreeMap::new(), "https://other.org/"));
    }

    #[test]
    fn matching_orders_least_specific_first() {
        let s = sites(&[("a.example.com", None), ("*", None), ("example.com", None)]);
        let keys: Vec<_> = matching(&s, "https://a.example.com/")
            .iter()
            .map(|(k, _)| *k)
            .collect();
        assert_eq!(keys, ["*", "example.com", "a.example.com"]);
    }
}
