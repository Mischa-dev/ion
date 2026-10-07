//! Origins and the site patterns rules are written against.

use std::fmt;
use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// A page's origin: scheme, host and port, the unit browsers grant
/// permissions to. Pages without a host (`file:`, `data:`, `about:blank`) have
/// an origin with `host == None`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct Origin {
    scheme: String,
    host: Option<String>,
    port: Option<u16>,
}

impl Origin {
    /// The origin of `url`. `None` if `url` does not parse.
    ///
    /// `blob:` and `filesystem:` URLs take the origin of the URL they wrap
    /// (`blob:https://a.example/id` is `https://a.example`), as browsers do.
    pub fn parse(url: &str) -> Option<Origin> {
        let url = url::Url::parse(url.trim()).ok()?;
        if matches!(url.scheme(), "blob" | "filesystem") {
            return Origin::parse(url.path());
        }
        Some(Origin {
            scheme: url.scheme().to_ascii_lowercase(),
            host: url
                .host_str()
                .filter(|h| !h.is_empty())
                .map(str::to_ascii_lowercase),
            // `port()` is `None` for the scheme's default port.
            port: url.port(),
        })
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    /// The host as people read it: `www.` dropped. `None` without a host.
    /// Whether pages with this origin can be told apart from each other:
    /// false for `data:`, `about:` and other hostless origins except local
    /// files, so an answer for one of them is never remembered for all.
    pub fn is_distinct(&self) -> bool {
        self.host.is_some() || self.scheme == "file"
    }

    pub fn display_host(&self) -> Option<&str> {
        self.host
            .as_deref()
            .map(|h| h.strip_prefix("www.").unwrap_or(h))
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://", self.scheme)?;
        if let Some(host) = &self.host {
            f.write_str(host)?;
        }
        if let Some(port) = self.port {
            write!(f, ":{port}")?;
        }
        Ok(())
    }
}

impl From<Origin> for String {
    fn from(origin: Origin) -> String {
        origin.to_string()
    }
}

impl TryFrom<String> for Origin {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Origin::parse(&value).ok_or_else(|| format!("not an origin: {value:?}"))
    }
}

/// Which sites a rule covers.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum SitePattern {
    /// Every site, and requests with no site at all (connectors, tab lists).
    Any,
    /// A host and all of its subdomains, any scheme and port: `github.com`
    /// covers `https://gist.github.com`. IP addresses match only themselves.
    Domain(String),
    /// Exactly one origin.
    Origin(Origin),
}

impl SitePattern {
    /// `"*"`, `"example.com"` (also written `"*.example.com"`), or a URL whose
    /// origin is meant (`"https://example.com"`).
    pub fn parse(text: &str) -> Option<SitePattern> {
        let text = text.trim();
        if text == "*" {
            return Some(SitePattern::Any);
        }
        if text.contains("://") {
            return Origin::parse(text).map(SitePattern::Origin);
        }
        let domain = text
            .strip_prefix("*.")
            .unwrap_or(text)
            .trim_matches('.')
            .to_ascii_lowercase();
        let valid = !domain.is_empty()
            && domain.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':' | '[' | ']')
            });
        valid.then_some(SitePattern::Domain(domain))
    }

    /// The pattern a prompt answer is remembered under: the page's host and
    /// its subdomains, or the exact origin for pages without a host.
    pub fn for_origin_host(origin: &Origin) -> SitePattern {
        match origin.host() {
            Some(host) => SitePattern::Domain(host.to_owned()),
            None => SitePattern::Origin(origin.clone()),
        }
    }

    /// Whether the pattern covers `site`. Only [`SitePattern::Any`] covers
    /// requests without a site.
    pub fn matches(&self, site: Option<&Origin>) -> bool {
        match (self, site) {
            (SitePattern::Any, _) => true,
            (_, None) => false,
            (SitePattern::Origin(o), Some(site)) => o == site,
            (SitePattern::Domain(d), Some(site)) => site.host().is_some_and(|host| {
                host == d
                    || (host.parse::<IpAddr>().is_err()
                        && !host.starts_with('[')
                        && host.len() > d.len()
                        && host.ends_with(d.as_str())
                        && host.as_bytes()[host.len() - d.len() - 1] == b'.')
            }),
        }
    }

    /// Higher is more specific: an origin beats any domain, a deeper domain
    /// beats a shallower one, and `Any` is least specific.
    pub fn specificity(&self) -> u32 {
        match self {
            SitePattern::Any => 0,
            SitePattern::Domain(d) => 1 + d.split('.').count() as u32,
            SitePattern::Origin(_) => 1000,
        }
    }
}

impl fmt::Display for SitePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SitePattern::Any => f.write_str("*"),
            SitePattern::Domain(d) => f.write_str(d),
            SitePattern::Origin(o) => o.fmt(f),
        }
    }
}

impl From<SitePattern> for String {
    fn from(pattern: SitePattern) -> String {
        pattern.to_string()
    }
}

impl TryFrom<String> for SitePattern {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        SitePattern::parse(&value).ok_or_else(|| format!("not a site pattern: {value:?}"))
    }
}

/// `url` with its user info, query and fragment removed, for the activity
/// log: keeps where an agent went without the tokens and search terms URLs
/// often carry.
pub fn loggable_url(url: &str) -> String {
    match url::Url::parse(url.trim()) {
        Ok(mut url) => {
            let _ = url.set_username("");
            let _ = url.set_password(None);
            url.set_query(None);
            url.set_fragment(None);
            url.to_string()
        }
        Err(_) => "[unparsable URL]".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(s: &str) -> Origin {
        Origin::parse(s).unwrap()
    }

    #[test]
    fn origins_drop_paths_and_default_ports() {
        assert_eq!(
            origin("https://GitHub.com:443/a/b?c#d").to_string(),
            "https://github.com"
        );
        assert_eq!(
            origin("http://localhost:8080/").to_string(),
            "http://localhost:8080"
        );
        assert_eq!(origin("file:///home/me/a.html").to_string(), "file://");
        assert_eq!(origin("file:///home/me/a.html").host(), None);
        assert_eq!(origin("about:blank").scheme(), "about");
        assert_eq!(Origin::parse("not a url"), None);
        assert_eq!(
            origin("blob:https://good.example/0b1c").to_string(),
            "https://good.example"
        );
        assert_eq!(origin("blob:chrome://settings/x").scheme(), "chrome");
        assert_eq!(
            origin("filesystem:http://a.example:8080/temporary/f").to_string(),
            "http://a.example:8080"
        );
        assert_eq!(Origin::parse("blob:nonsense"), None);
    }

    #[test]
    fn display_host_drops_www() {
        assert_eq!(
            origin("https://www.example.com").display_host(),
            Some("example.com")
        );
        assert_eq!(
            origin("https://mail.example.com").display_host(),
            Some("mail.example.com")
        );
        assert_eq!(origin("file:///x").display_host(), None);
    }

    #[test]
    fn patterns_parse() {
        assert_eq!(SitePattern::parse("*"), Some(SitePattern::Any));
        assert_eq!(
            SitePattern::parse("*.GitHub.com"),
            Some(SitePattern::Domain("github.com".into()))
        );
        assert_eq!(
            SitePattern::parse("https://github.com/x"),
            Some(SitePattern::Origin(origin("https://github.com")))
        );
        assert_eq!(SitePattern::parse(""), None);
        assert_eq!(SitePattern::parse("a/b"), None);
    }

    #[test]
    fn domains_cover_subdomains_only_on_label_boundaries() {
        let gh = SitePattern::parse("github.com").unwrap();
        assert!(gh.matches(Some(&origin("https://github.com"))));
        assert!(gh.matches(Some(&origin("http://gist.github.com:81"))));
        assert!(!gh.matches(Some(&origin("https://notgithub.com"))));
        assert!(!gh.matches(Some(&origin("https://github.com.evil.example"))));
        assert!(!gh.matches(Some(&origin("file:///github.com"))));
        assert!(!gh.matches(None));
    }

    #[test]
    fn ip_domains_match_exactly() {
        let ip = SitePattern::parse("2.3.4").unwrap();
        assert!(!ip.matches(Some(&origin("http://1.2.3.4"))));
        let ip = SitePattern::parse("1.2.3.4").unwrap();
        assert!(ip.matches(Some(&origin("http://1.2.3.4:8000"))));
    }

    #[test]
    fn origin_patterns_are_exact() {
        let p = SitePattern::parse("https://github.com").unwrap();
        assert!(p.matches(Some(&origin("https://github.com/login"))));
        assert!(!p.matches(Some(&origin("http://github.com"))));
        assert!(!p.matches(Some(&origin("https://gist.github.com"))));
    }

    #[test]
    fn any_matches_everything_including_no_site() {
        assert!(SitePattern::Any.matches(None));
        assert!(SitePattern::Any.matches(Some(&origin("file:///x"))));
    }

    #[test]
    fn specificity_orders_patterns() {
        let any = SitePattern::Any.specificity();
        let shallow = SitePattern::parse("github.com").unwrap().specificity();
        let deep = SitePattern::parse("gist.github.com").unwrap().specificity();
        let exact = SitePattern::parse("https://github.com")
            .unwrap()
            .specificity();
        assert!(any < shallow && shallow < deep && deep < exact);
    }

    #[test]
    fn serde_round_trips_as_strings() {
        let p = SitePattern::parse("github.com").unwrap();
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"github.com\"");
        assert_eq!(serde_json::from_str::<SitePattern>(&json).unwrap(), p);
        assert!(serde_json::from_str::<SitePattern>("\"a/b\"").is_err());
    }

    #[test]
    fn loggable_urls_lose_secrets() {
        assert_eq!(
            loggable_url("https://me:pw@example.com/a/b?token=abc#frag"),
            "https://example.com/a/b"
        );
        assert_eq!(loggable_url("nope"), "[unparsable URL]");
    }
}
