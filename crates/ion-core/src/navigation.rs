//! Turning what someone typed into the URL bar into something to load.
//!
//! The URL bar is a command line: later stages (bangs, Ion commands, `!ai`)
//! plug in as extra steps in [`Omnibox::resolve`] before the final
//! "is it a URL, or is it a search?" decision made here.

use std::net::IpAddr;

use url::Url;

/// Schemes that are always passed through untouched.
const KNOWN_SCHEMES: &[&str] = &[
    "http",
    "https",
    "file",
    "about",
    "data",
    "chrome",
    "view-source",
    "ion",
    "mailto",
];

/// A search engine described by a URL template containing `{}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchEngine {
    pub name: String,
    pub template: String,
}

impl SearchEngine {
    pub fn new(name: impl Into<String>, template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            template: template.into(),
        }
    }

    /// Build the search URL for `query`, percent-encoding it.
    pub fn url_for(&self, query: &str) -> String {
        let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
        self.template.replace("{}", &encoded)
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new("DuckDuckGo", "https://duckduckgo.com/?q={}")
    }
}

/// What the URL bar decided to do with an input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// Navigate to an address the person typed.
    Url(String),
    /// Run a web search; the string is the full search URL.
    Search(String),
}

impl Resolved {
    pub fn url(&self) -> &str {
        match self {
            Resolved::Url(u) | Resolved::Search(u) => u,
        }
    }
}

/// Resolves URL-bar input.
#[derive(Debug, Clone, Default)]
pub struct Omnibox {
    pub search: SearchEngine,
}

impl Omnibox {
    pub fn new(search: SearchEngine) -> Self {
        Self { search }
    }

    /// Resolve `input` into something loadable. Returns `None` for blank input.
    pub fn resolve(&self, input: &str) -> Option<Resolved> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }
        if let Some(url) = as_url(input) {
            return Some(Resolved::Url(url));
        }
        Some(Resolved::Search(self.search.url_for(input)))
    }
}

/// Returns the normalized URL if `input` looks like an address rather than a search.
fn as_url(input: &str) -> Option<String> {
    if input.chars().any(char::is_whitespace) {
        return None;
    }

    // Explicit scheme: `https://…`, `about:blank`, `file:///…`.
    if let Some((scheme, _)) = input.split_once(':') {
        if KNOWN_SCHEMES.contains(&scheme.to_ascii_lowercase().as_str()) {
            return Url::parse(input).ok().map(String::from);
        }
    }

    // Absolute local path.
    if input.starts_with('/') {
        return Url::from_file_path(input).ok().map(String::from);
    }

    let (host, _) = split_host(input);
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let local = host.eq_ignore_ascii_case("localhost") || host.parse::<IpAddr>().is_ok();
    let looks_like_domain = host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || !c.is_ascii())
        && host
            .rsplit('.')
            .next()
            .is_some_and(|tld| tld.chars().any(|c| !c.is_ascii_digit()));

    if !(local || looks_like_domain) {
        return None;
    }

    let scheme = if local { "http" } else { "https" };
    Url::parse(&format!("{scheme}://{input}"))
        .ok()
        .map(String::from)
}

/// Split `host[:port][/path…]` into the host part and the rest.
fn split_host(input: &str) -> (&str, &str) {
    let end = input.find(['/', '?', '#']).unwrap_or(input.len());
    let authority = &input[..end];
    let host = if authority.starts_with('[') {
        authority.split_inclusive(']').next().unwrap_or(authority)
    } else {
        authority.rsplit_once(':').map_or(authority, |(h, port)| {
            if port.chars().all(|c| c.is_ascii_digit()) {
                h
            } else {
                authority
            }
        })
    };
    (host, &input[end..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(input: &str) -> Option<Resolved> {
        Omnibox::default().resolve(input)
    }

    fn url(s: &str) -> Option<Resolved> {
        Some(Resolved::Url(s.to_owned()))
    }

    fn search(q: &str) -> Option<Resolved> {
        Some(Resolved::Search(SearchEngine::default().url_for(q)))
    }

    #[test]
    fn blank_input_is_ignored() {
        assert_eq!(resolve(""), None);
        assert_eq!(resolve("   "), None);
    }

    #[test]
    fn explicit_urls_pass_through() {
        assert_eq!(resolve("https://example.com"), url("https://example.com/"));
        assert_eq!(resolve("about:blank"), url("about:blank"));
        assert_eq!(resolve("file:///etc/hosts"), url("file:///etc/hosts"));
        assert_eq!(
            resolve("  HTTP://Example.com/a  "),
            url("http://example.com/a")
        );
    }

    #[test]
    fn bare_domains_get_https() {
        assert_eq!(resolve("example.com"), url("https://example.com/"));
        assert_eq!(
            resolve("github.com/Mischa-dev/ion?tab=readme"),
            url("https://github.com/Mischa-dev/ion?tab=readme")
        );
        assert_eq!(
            resolve("search.nixos.org:443"),
            url("https://search.nixos.org/")
        );
    }

    #[test]
    fn local_hosts_get_http() {
        assert_eq!(resolve("localhost"), url("http://localhost/"));
        assert_eq!(resolve("localhost:8080/x"), url("http://localhost:8080/x"));
        assert_eq!(resolve("127.0.0.1:3000"), url("http://127.0.0.1:3000/"));
        assert_eq!(resolve("[::1]:8000"), url("http://[::1]:8000/"));
    }

    #[test]
    fn absolute_paths_become_file_urls() {
        assert_eq!(resolve("/tmp/a.html"), url("file:///tmp/a.html"));
    }

    #[test]
    fn everything_else_is_a_search() {
        assert_eq!(resolve("rust cxx-qt"), search("rust cxx-qt"));
        assert_eq!(resolve("nixos"), search("nixos"));
        assert_eq!(resolve("1.5"), search("1.5"));
        assert_eq!(
            resolve("what is example.com"),
            search("what is example.com")
        );
        assert_eq!(resolve("foo:bar"), search("foo:bar"));
    }

    #[test]
    fn search_queries_are_encoded() {
        assert_eq!(
            SearchEngine::default().url_for("a&b c"),
            "https://duckduckgo.com/?q=a%26b+c"
        );
    }
}
