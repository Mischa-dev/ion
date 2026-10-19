//! Which saved logins belong to a page.

/// The origin a login is saved under: `scheme://host[:port]`, lower-cased,
/// default ports dropped. `None` for anything but http(s) pages.
pub fn origin_of(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url.trim()).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    let host = parsed
        .host_str()?
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    Some(match parsed.port() {
        Some(port) => format!("{}://{host}:{port}", parsed.scheme()),
        None => format!("{}://{host}", parsed.scheme()),
    })
}

/// The host of an origin, for showing to people: `https://www.github.com`
/// shows as `github.com`.
pub fn display_host(origin: &str) -> String {
    let rest = origin.split_once("://").map_or(origin, |(_, rest)| rest);
    rest.strip_prefix("www.").unwrap_or(rest).to_owned()
}

/// The registrable domain ("eTLD+1") of an origin's host: `accounts.google.com`
/// and `mail.google.com` share `google.com`. Hosts without one (IP addresses,
/// `localhost`) are their own site.
pub fn site_of(origin: &str) -> String {
    let host = host_of(origin);
    match psl::domain(host.as_bytes()) {
        Some(domain) if domain.suffix().is_known() => {
            String::from_utf8_lossy(domain.as_bytes()).into_owned()
        }
        _ => host.to_owned(),
    }
}

fn host_of(origin: &str) -> &str {
    let rest = origin.split_once("://").map_or(origin, |(_, rest)| rest);
    // Strip a port; IPv6 hosts keep their brackets.
    match rest.rfind(':') {
        Some(i) if !rest[i..].contains(']') => &rest[..i],
        _ => rest,
    }
}

/// How well a saved login's origin fits a page's origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fit {
    /// Another subdomain of the same site: offered, never filled by itself.
    SameSite,
    /// The page's own origin, or its http version seen over https.
    Exact,
}

/// How `saved` fits a page at `page` (both origins), if at all.
pub fn fit(saved: &str, page: &str) -> Option<Fit> {
    if saved == page {
        return Some(Fit::Exact);
    }
    // A login saved on the http site also works once the site moved to
    // https, never the other way round.
    if let (Some(saved_rest), Some(page_rest)) =
        (saved.strip_prefix("http://"), page.strip_prefix("https://"))
        && saved_rest == page_rest
    {
        return Some(Fit::Exact);
    }
    // Otherwise the schemes must match: an https login never goes to a
    // plain-http page, and an https page never takes one from a plain-http
    // subdomain.
    let scheme = |o: &str| o.split_once("://").map(|(s, _)| s.to_owned());
    if scheme(saved) != scheme(page) || !page.starts_with("http") {
        return None;
    }
    let site = site_of(page);
    // Only real registrable domains group subdomains; two IPs or two
    // intranet names don't.
    let grouped = psl::domain(host_of(page).as_bytes()).is_some_and(|d| d.suffix().is_known());
    (grouped && site_of(saved) == site).then_some(Fit::SameSite)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_are_normalized() {
        assert_eq!(
            origin_of("https://Accounts.Google.com/signin?x=1").as_deref(),
            Some("https://accounts.google.com")
        );
        assert_eq!(
            origin_of("http://example.com:80/").as_deref(),
            Some("http://example.com")
        );
        assert_eq!(
            origin_of("http://localhost:8080/login").as_deref(),
            Some("http://localhost:8080")
        );
        assert_eq!(origin_of("file:///tmp/a.html"), None);
        assert_eq!(origin_of("about:blank"), None);
    }

    #[test]
    fn sites_group_subdomains() {
        assert_eq!(site_of("https://accounts.google.com"), "google.com");
        assert_eq!(site_of("https://a.b.example.co.uk:8443"), "example.co.uk");
        assert_eq!(site_of("http://localhost:8080"), "localhost");
        assert_eq!(display_host("https://www.github.com"), "github.com");
    }

    #[test]
    fn fit_prefers_the_exact_origin() {
        assert_eq!(fit("https://a.com", "https://a.com"), Some(Fit::Exact));
        assert_eq!(fit("http://a.com", "https://a.com"), Some(Fit::Exact));
        assert_eq!(fit("https://a.com", "http://a.com"), None);
        assert_eq!(
            fit("https://login.a.com", "https://a.com"),
            Some(Fit::SameSite)
        );
        assert_eq!(fit("http://login.a.com", "https://a.com"), None);
        assert_eq!(fit("https://b.com", "https://a.com"), None);
        // github.io is a public suffix: users' pages are separate sites.
        assert_eq!(
            fit("https://alice.github.io", "https://bob.github.io"),
            None
        );
        assert_eq!(fit("http://localhost:1", "http://localhost:2"), None);
        assert_eq!(fit("android://x@com.app/", "https://a.com"), None);
    }
}
