//! Site search engines found on the sites you visit.
//!
//! Many sites describe their own search in an OpenSearch file
//! (`<link rel="search" type="application/opensearchdescription+xml">`).
//! Ion reads it once per site and remembers the search, so typing the site's
//! name and pressing Tab searches it, the way Chrome's site search works.
//! Each found engine becomes a bang (`!youtube`, `!docs.rs`) alongside the
//! built-in ones, never replacing a bang that already exists.

use std::net::IpAddr;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Sites remembered at most; the oldest are dropped first.
pub const MAX_ENGINES: usize = 300;

/// Redirects followed at most while fetching a description.
const MAX_HOPS: usize = 3;

/// Descriptions are a few KB; anything far bigger isn't one.
const MAX_DESCRIPTION_BYTES: u64 = 256 * 1024;

/// One site's search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteEngine {
    /// The site's host without `www.`.
    pub host: String,
    /// The site's own name for its search ("YouTube").
    pub name: String,
    /// Results page with `{}` where the query goes.
    pub template: String,
}

impl SiteEngine {
    /// What can follow the `!` (or be typed before Tab) to use it: the host
    /// (`en.wikipedia.org`) and its site name (`wikipedia`).
    pub fn triggers(&self) -> Vec<String> {
        let mut triggers = vec![self.host.clone()];
        if let Some(label) = site_label(&self.host)
            && label != self.host
        {
            triggers.push(label.to_owned());
        }
        triggers
    }
}

/// The name part of a host: `wikipedia` for `en.wikipedia.org`, `bbc` for
/// `news.bbc.co.uk`, `project` for `project.github.io`. Public-suffix aware,
/// so shared hosts don't hand one tenant the platform's name.
pub fn site_label(host: &str) -> Option<&str> {
    let label = registrable_domain(host)?.split('.').next()?;
    (label.len() >= 3).then_some(label)
}

/// The part of `host` one owner registered (`bbc.co.uk`), or `None` for IP
/// addresses and bare suffixes.
fn registrable_domain(host: &str) -> Option<&str> {
    if host.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }
    psl::domain_str(host)
}

/// The site's search from an OpenSearch description `xml` found on the page
/// at `page_url`. Like [`parse`], and an HTTPS page's search has to stay on
/// HTTPS, so learning it never sends your words in the clear.
pub fn parse_for_page(xml: &str, page_url: &str) -> Option<SiteEngine> {
    let page = url::Url::parse(page_url).ok()?;
    parse_with(xml, page.host_str()?, page.scheme() == "https")
}

/// The site's search from an OpenSearch description `xml` fetched for a page
/// on `page_host`. The results page has to be on the same site.
pub fn parse(xml: &str, page_host: &str) -> Option<SiteEngine> {
    parse_with(xml, page_host, false)
}

/// [`parse`], taking only HTTPS results pages when `https_only`.
fn parse_with(xml: &str, page_host: &str, https_only: bool) -> Option<SiteEngine> {
    let page_host = page_host.trim_start_matches("www.").to_lowercase();
    let name = element_text(xml, "ShortName")
        .map(|name| decode(name.trim()))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| page_host.clone());

    let language = element_text(xml, "Language")
        .map(str::trim)
        .filter(|l| {
            !l.is_empty()
                && l.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'*')
        })
        .unwrap_or("*");
    // The first results page that works; a broken or off-site one doesn't
    // stop a later one from being used.
    let template = tags(xml, "Url").find_map(|tag| {
        let kind = attribute(tag, "type").unwrap_or("text/html");
        if kind != "text/html" {
            return None;
        }
        let template = decode(attribute(tag, "template")?);
        if !template.contains("{searchTerms}") {
            return None;
        }
        let template = template.replace("{searchTerms}", "\u{0}");
        let number = |name| {
            attribute(tag, name)
                .filter(|n: &&str| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                .unwrap_or("1")
        };
        let offsets = Offsets {
            index: number("indexOffset"),
            page: number("pageOffset"),
            language,
        };
        let template = fill_parameters(&template, &offsets)?.replace('\u{0}', "{}");
        let url = url::Url::parse(&template.replace("{}", "q")).ok()?;
        let allowed: &[&str] = if https_only {
            &["https"]
        } else {
            &["https", "http"]
        };
        if !allowed.contains(&url.scheme()) {
            return None;
        }
        let host = url.host_str()?.trim_start_matches("www.").to_lowercase();
        same_site(&host, &page_host).then_some(template)
    })?;

    Some(SiteEngine {
        host: page_host,
        name,
        template,
    })
}

/// Whether Ion may fetch the OpenSearch description at `description`, linked
/// from the page at `page`: only from the page's own site, and not over plain
/// HTTP from an HTTPS page. A page can link anything, and the fetch isn't
/// held to the web's origin rules, so this keeps it from reaching other hosts.
pub fn may_fetch(page: &str, description: &str) -> bool {
    let (Ok(page), Ok(description)) = (url::Url::parse(page), url::Url::parse(description)) else {
        return false;
    };
    let secure_enough = match (page.scheme(), description.scheme()) {
        ("https", scheme) => scheme == "https",
        ("http", scheme) => matches!(scheme, "http" | "https"),
        _ => false,
    };
    let (Some(page_host), Some(host)) = (page.host_str(), description.host_str()) else {
        return false;
    };
    secure_enough && same_site(&page_host.to_lowercase(), &host.to_lowercase())
}

/// Fetches the OpenSearch description at `description`, linked from `page`.
/// Every address on the way, each redirect included, has to pass
/// [`may_fetch`] before it is requested, and unless the page itself is on a
/// local network the names have to resolve to public addresses, so a site
/// can't point Ion at your router through its own DNS. Blocks; sends no
/// cookies and uses no proxy (a proxy would resolve the names itself).
pub fn fetch(page: &str, description: &str) -> Option<String> {
    let local = url::Url::parse(page)
        .ok()
        .and_then(|url| url.host_str().map(is_local_host))
        .unwrap_or(false);
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .max_redirects(0)
        .http_status_as_error(false)
        .proxy(None)
        .user_agent(concat!("Ion/", env!("CARGO_PKG_VERSION")))
        .build();
    let agent = ureq::Agent::with_parts(
        config,
        ureq::unversioned::transport::DefaultConnector::default(),
        Resolve { local },
    );
    let mut address = url::Url::parse(description).ok()?;
    for _ in 0..=MAX_HOPS {
        if !may_fetch(page, address.as_str()) {
            return None;
        }
        let mut response = agent.get(address.as_str()).call().ok()?;
        let status = response.status();
        if status.is_redirection() {
            let location = response.headers().get("location")?.to_str().ok()?;
            address = address.join(location).ok()?;
            continue;
        }
        if status != 200 {
            return None;
        }
        return response
            .body_mut()
            .with_config()
            .limit(MAX_DESCRIPTION_BYTES)
            .read_to_string()
            .ok();
    }
    None
}

/// Resolves names for [`fetch`], keeping only public addresses unless the
/// page was `local`. ureq connects to exactly what this returns.
#[derive(Debug)]
struct Resolve {
    local: bool,
}

impl ureq::unversioned::resolver::Resolver for Resolve {
    fn resolve(
        &self,
        uri: &ureq::http::Uri,
        config: &ureq::config::Config,
        timeout: ureq::unversioned::transport::NextTimeout,
    ) -> Result<ureq::unversioned::resolver::ResolvedSocketAddrs, ureq::Error> {
        let found = ureq::unversioned::resolver::DefaultResolver::default()
            .resolve(uri, config, timeout)?;
        let mut kept = self.empty();
        for address in found.iter().filter(|a| self.local || is_public(a.ip())) {
            kept.push(*address);
        }
        if kept.is_empty() {
            return Err(ureq::Error::HostNotFound);
        }
        Ok(kept)
    }
}

/// Whether `host` names a machine on a local network: a non-public IP,
/// `localhost`, a bare name, or a local-only domain.
fn is_local_host(host: &str) -> bool {
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_lowercase();
    if let Ok(ip) = host.parse::<IpAddr>() {
        return !is_public(ip);
    }
    !host.contains('.')
        || [".localhost", ".local", ".internal", ".lan", ".home.arpa"]
            .iter()
            .any(|suffix| host.ends_with(suffix))
}

/// Whether `ip` is reachable on the public internet (not loopback, private,
/// link-local, shared, documentation, multicast or reserved).
fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, c, _] = v4.octets();
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_multicast()
                || a == 0
                || a >= 240
                || (a == 100 && (64..128).contains(&b))
                || (a == 192 && b == 0 && c == 0)
                || (a == 198 && (18..20).contains(&b)))
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public(IpAddr::V4(v4));
            }
            let first = v6.segments()[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (first & 0xfe00) == 0xfc00
                || (first & 0xffc0) == 0xfe80
                || (first & 0xffc0) == 0xfec0
                || (first == 0x2001 && v6.segments()[1] == 0x0db8))
        }
    }
}

/// Whether two hosts belong to one site: equal, or one a subdomain of the
/// other's registrable part.
fn same_site(a: &str, b: &str) -> bool {
    a == b || registrable_domain(a).is_some_and(|base| registrable_domain(b) == Some(base))
}

/// What Ion sends for OpenSearch's standard required parameters, besides
/// the language and first result and page numbers, which come from
/// [`Offsets`].
const PARAMETER_DEFAULTS: &[(&str, &str)] = &[
    ("inputEncoding", "UTF-8"),
    ("outputEncoding", "UTF-8"),
    ("count", "20"),
];

/// A `<Url>`'s first result and page numbers (`indexOffset`, `pageOffset`),
/// and the description's first `<Language>` (`*`, any, when it names none).
struct Offsets<'a> {
    index: &'a str,
    page: &'a str,
    language: &'a str,
}

/// Fills the parameters left in `template` other than the query: query
/// pairs holding only optional ones (`{startPage?}`) go, standard required
/// ones get their defaults, and an unknown required one means Ion can't use
/// the template.
fn fill_parameters(template: &str, offsets: &Offsets) -> Option<String> {
    let template = match template.split_once('?') {
        Some((base, query)) => {
            let kept: Vec<&str> = query
                .split('&')
                .filter(|pair| !only_optional_parameters(pair))
                .collect();
            if kept.is_empty() {
                base.to_owned()
            } else {
                format!("{base}?{}", kept.join("&"))
            }
        }
        None => template.to_owned(),
    };
    let mut filled = String::with_capacity(template.len());
    let mut rest = template.as_str();
    while let Some(start) = rest.find('{') {
        let end = start + rest[start..].find('}')?;
        filled.push_str(&rest[..start]);
        let name = &rest[start + 1..end];
        // An optional parameter outside the query is just left empty.
        if !name.ends_with('?') {
            let value = match name {
                "startIndex" => offsets.index,
                "startPage" => offsets.page,
                "language" => offsets.language,
                _ => PARAMETER_DEFAULTS
                    .iter()
                    .find(|(known, _)| *known == name)
                    .map(|(_, value)| *value)?,
            };
            filled.push_str(value);
        }
        rest = &rest[end + 1..];
    }
    filled.push_str(rest);
    Some(filled)
}

/// Whether every parameter in a query pair is optional (`{name?}`).
fn only_optional_parameters(pair: &str) -> bool {
    let mut any = false;
    for piece in pair.split('{').skip(1) {
        let Some((name, _)) = piece.split_once('}') else {
            return false;
        };
        if !name.ends_with('?') {
            return false;
        }
        any = true;
    }
    any
}

/// Opening tags named `name` (with or without a namespace prefix).
fn tags<'a>(xml: &'a str, name: &'a str) -> impl Iterator<Item = &'a str> {
    xml.split('<').skip(1).filter_map(move |chunk| {
        let tag = chunk.split('>').next()?;
        let tag_name = tag.split_whitespace().next()?.trim_end_matches('/');
        let local = tag_name.rsplit(':').next()?;
        (local == name).then_some(tag)
    })
}

fn element_text<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let start = xml.find(&format!("<{name}"))?;
    let after = &xml[start..];
    let open_end = after.find('>')? + 1;
    let rest = &after[open_end..];
    let close = rest.find("</")?;
    Some(&rest[..close])
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = tag;
    while let Some(at) = rest.find(name) {
        let before_ok = rest[..at]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace);
        let after = rest[at + name.len()..].trim_start();
        if before_ok && let Some(value) = after.strip_prefix('=') {
            let value = value.trim_start();
            let quote = value.chars().next()?;
            if quote == '"' || quote == '\'' {
                let value = &value[1..];
                return value.find(quote).map(|end| &value[..end]);
            }
        }
        rest = &rest[at + name.len()..];
    }
    None
}

fn decode(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

/// The engines found so far, newest last.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteEngines {
    #[serde(default)]
    engines: Vec<SiteEngine>,
}

impl SiteEngines {
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_owned())
    }

    pub fn iter(&self) -> impl Iterator<Item = &SiteEngine> {
        self.engines.iter()
    }

    pub fn len(&self) -> usize {
        self.engines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.engines.is_empty()
    }

    /// Whether `host` already has an engine.
    pub fn knows(&self, host: &str) -> bool {
        let host = host.trim_start_matches("www.");
        self.engines.iter().any(|e| e.host == host)
    }

    /// Add or update a site's engine. True if anything changed.
    pub fn add(&mut self, engine: SiteEngine) -> bool {
        if let Some(at) = self.engines.iter().position(|e| e.host == engine.host) {
            if self.engines[at] == engine {
                return false;
            }
            self.engines.remove(at);
        }
        self.engines.push(engine);
        if self.engines.len() > MAX_ENGINES {
            self.engines.remove(0);
        }
        true
    }

    pub fn clear(&mut self) {
        self.engines.clear();
    }

    /// Drop the engines of `site` and its subdomains. True if any went.
    pub fn forget_site(&mut self, site: &str) -> bool {
        let site = site.trim().trim_start_matches("www.").to_lowercase();
        if site.is_empty() {
            return false;
        }
        let before = self.engines.len();
        self.engines
            .retain(|e| e.host != site && !e.host.ends_with(&format!(".{site}")));
        self.engines.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YOUTUBE: &str = r#"<?xml version="1.0"?>
<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/">
  <ShortName>YouTube</ShortName>
  <Url type="text/html" method="get" template="https://www.youtube.com/results?search_query={searchTerms}&amp;page={startPage?}&amp;utm_source=opensearch"/>
  <Url type="application/x-suggestions+json" template="https://suggest.example/?q={searchTerms}"/>
</OpenSearchDescription>"#;

    #[test]
    fn forgetting_a_site_takes_its_subdomains() {
        let mut engines = SiteEngines::default();
        for host in ["youtube.com", "music.youtube.com", "notyoutube.com"] {
            engines.add(SiteEngine {
                name: host.into(),
                host: host.into(),
                template: format!("https://{host}/?q={{}}"),
            });
        }
        assert!(engines.forget_site("www.YouTube.com"));
        let left: Vec<&str> = engines.iter().map(|e| e.host.as_str()).collect();
        assert_eq!(left, ["notyoutube.com"]);
        assert!(!engines.forget_site("youtube.com"));
        assert!(!engines.forget_site(""));
    }

    #[test]
    fn reads_a_description() {
        let engine = parse(YOUTUBE, "www.youtube.com").unwrap();
        assert_eq!(engine.name, "YouTube");
        assert_eq!(engine.host, "youtube.com");
        assert_eq!(
            engine.template,
            "https://www.youtube.com/results?search_query={}&utm_source=opensearch"
        );
        assert_eq!(engine.triggers(), ["youtube.com", "youtube"]);
    }

    #[test]
    fn handles_prefixes_quotes_and_missing_names() {
        let xml = "<os:OpenSearchDescription><os:Url template='https://docs.rs/releases/search?query={searchTerms}' type='text/html'/></os:OpenSearchDescription>";
        let engine = parse(xml, "docs.rs").unwrap();
        assert_eq!(engine.name, "docs.rs");
        assert_eq!(engine.template, "https://docs.rs/releases/search?query={}");
        assert_eq!(engine.triggers(), ["docs.rs", "docs"]);
    }

    #[test]
    fn subdomains_share_a_site() {
        let xml = r#"<ShortName>Wikipedia (en)</ShortName><Url type="text/html" template="https://en.wikipedia.org/w/index.php?search={searchTerms}"/>"#;
        let engine = parse(xml, "en.m.wikipedia.org").unwrap();
        assert_eq!(engine.triggers(), ["en.m.wikipedia.org", "wikipedia"]);
    }

    #[test]
    fn refuses_other_sites_and_schemes() {
        let elsewhere =
            r#"<Url type="text/html" template="https://evil.example/?q={searchTerms}"/>"#;
        assert_eq!(parse(elsewhere, "youtube.com"), None);
        let script = r#"<Url type="text/html" template="javascript:alert('{searchTerms}')"/>"#;
        assert_eq!(parse(script, "youtube.com"), None);
        let no_query = r#"<Url type="text/html" template="https://youtube.com/"/>"#;
        assert_eq!(parse(no_query, "youtube.com"), None);
        assert_eq!(parse("not xml", "youtube.com"), None);
    }

    #[test]
    fn descriptions_are_fetched_only_from_the_pages_own_site() {
        let page = "https://en.wikipedia.org/wiki/Rust";
        assert!(may_fetch(
            page,
            "https://en.wikipedia.org/w/opensearch_desc.php"
        ));
        assert!(may_fetch(page, "https://wikipedia.org/opensearch.xml"));
        assert!(!may_fetch(page, "http://en.wikipedia.org/opensearch.xml"));
        assert!(!may_fetch(page, "https://127.0.0.1/opensearch.xml"));
        assert!(!may_fetch(page, "https://intranet.local/opensearch.xml"));
        assert!(!may_fetch(page, "https://evil.example/opensearch.xml"));
        assert!(!may_fetch(
            "https://a.github.io/",
            "https://b.github.io/os.xml"
        ));
        assert!(may_fetch(
            "http://localhost:8000/",
            "http://localhost:8000/os.xml"
        ));
        assert!(!may_fetch("http://10.0.0.1/", "http://10.0.0.2/os.xml"));
        assert!(!may_fetch(page, "file:///etc/passwd"));
    }

    /// Serves `responses` in order on a local port, one per connection,
    /// and returns the address and how many requests it got.
    fn serve(responses: Vec<String>) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        use std::io::{Read, Write};
        use std::sync::atomic::Ordering;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = hits.clone();
        std::thread::spawn(move || {
            for response in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                counter.fetch_add(1, Ordering::SeqCst);
                let mut request = [0; 4096];
                let _ = stream.read(&mut request);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (address, hits)
    }

    fn reply(status: &str, headers: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    #[test]
    fn fetches_a_description_through_same_site_redirects() {
        let xml = "<OpenSearchDescription/>";
        let (base, hits) = serve(vec![
            reply("302 Found", "Location: /real.xml\r\n", ""),
            reply("200 OK", "", xml),
        ]);
        let page = format!("{base}/");
        assert_eq!(
            fetch(&page, &format!("{base}/os.xml")).as_deref(),
            Some(xml)
        );
        assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn never_follows_a_redirect_off_the_site() {
        let (base, _) = serve(vec![reply(
            "302 Found",
            "Location: http://10.0.0.1/admin/reboot\r\n",
            "",
        )]);
        let page = format!("{base}/");
        // 10.0.0.1 is never contacted: the hop is refused before any request.
        assert_eq!(fetch(&page, &format!("{base}/os.xml")), None);
    }

    #[test]
    fn only_public_addresses_count_as_public() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.1.1",
            "100.64.0.1",
            "0.0.0.0",
            "::1",
            "fd00::1",
            "fe80::1",
            "::ffff:192.168.1.1",
        ] {
            assert!(!is_public(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["1.1.1.1", "93.184.216.34", "2606:4700::1111"] {
            assert!(is_public(ip.parse().unwrap()), "{ip}");
        }
        assert!(is_local_host("localhost"));
        assert!(is_local_host("printer.local"));
        assert!(is_local_host("192.168.1.10"));
        assert!(is_local_host("nas"));
        assert!(!is_local_host("en.wikipedia.org"));
        assert!(!is_local_host("1.1.1.1"));
    }

    #[test]
    fn public_pages_never_reach_local_addresses() {
        // A site's DNS can point anywhere; what matters is the address the
        // name resolves to, checked before anything is sent.
        let resolve = Resolve { local: false };
        let uri: ureq::http::Uri = "http://127.0.0.1:9/".parse().unwrap();
        let config = ureq::config::Config::default();
        let timeout = ureq::unversioned::transport::NextTimeout {
            after: ureq::unversioned::transport::time::Duration::NotHappening,
            reason: ureq::Timeout::Global,
        };
        use ureq::unversioned::resolver::Resolver;
        assert!(resolve.resolve(&uri, &config, timeout).is_err());
        assert!(
            Resolve { local: true }
                .resolve(&uri, &config, timeout)
                .is_ok()
        );
    }

    #[test]
    fn a_secure_page_never_learns_a_plain_http_search() {
        let xml = r#"<OpenSearchDescription><ShortName>Docs</ShortName>
            <Url type="text/html" template="http://docs.example.com/s?q={searchTerms}"/>
            </OpenSearchDescription>"#;
        assert_eq!(parse_for_page(xml, "https://docs.example.com/"), None);
        let engine = parse_for_page(xml, "http://docs.example.com/").unwrap();
        assert_eq!(engine.template, "http://docs.example.com/s?q={}");
    }

    #[test]
    fn required_parameters_get_their_defaults() {
        let xml = r#"<OpenSearchDescription><Url type="text/html"
            template="https://example.com/s?q={searchTerms}&amp;lang={language}&amp;page={startPage?}&amp;ie={inputEncoding}"/>
            </OpenSearchDescription>"#;
        let engine = parse(xml, "example.com").unwrap();
        assert_eq!(
            engine.template,
            "https://example.com/s?q={}&lang=*&ie=UTF-8"
        );
        let unknown = r#"<OpenSearchDescription><Url type="text/html"
            template="https://example.com/s?q={searchTerms}&amp;key={custom:apiKey}"/>
            </OpenSearchDescription>"#;
        assert_eq!(parse(unknown, "example.com"), None);
    }

    #[test]
    fn a_secure_page_skips_a_plain_http_url_for_a_later_https_one() {
        let xml = r#"<OpenSearchDescription>
            <Url type="text/html" template="http://example.com/s?q={searchTerms}"/>
            <Url type="text/html" template="https://example.com/s?q={searchTerms}"/>
            </OpenSearchDescription>"#;
        let engine = parse_for_page(xml, "https://example.com/").unwrap();
        assert_eq!(engine.template, "https://example.com/s?q={}");
    }

    #[test]
    fn first_result_and_page_numbers_follow_the_urls_offsets() {
        let xml = r#"<OpenSearchDescription><Url type="text/html" indexOffset="0"
            template="https://example.com/s?q={searchTerms}&amp;start={startIndex}&amp;p={startPage}"/>
            </OpenSearchDescription>"#;
        let engine = parse(xml, "example.com").unwrap();
        assert_eq!(engine.template, "https://example.com/s?q={}&start=0&p=1");
    }

    #[test]
    fn the_language_is_one_the_description_names() {
        let xml = r#"<OpenSearchDescription><Language>de</Language><Url type="text/html"
            template="https://example.com/s?q={searchTerms}&amp;hl={language}"/>
            </OpenSearchDescription>"#;
        let engine = parse(xml, "example.com").unwrap();
        assert_eq!(engine.template, "https://example.com/s?q={}&hl=de");
    }

    #[test]
    fn a_later_url_is_used_when_an_earlier_one_is_unusable() {
        let xml = r#"<OpenSearchDescription>
            <Url type="text/html" template="https://elsewhere.example/s?q={searchTerms}"/>
            <Url type="text/html" template="https://example.com/s?q={searchTerms}"/>
            </OpenSearchDescription>"#;
        let engine = parse(xml, "example.com").unwrap();
        assert_eq!(engine.template, "https://example.com/s?q={}");
    }

    #[test]
    fn site_labels_follow_the_public_suffix_list() {
        assert_eq!(site_label("news.bbc.co.uk"), Some("bbc"));
        assert_eq!(site_label("en.wikipedia.org"), Some("wikipedia"));
        assert_eq!(site_label("github.com"), Some("github"));
        // A GitHub Pages site isn't GitHub.
        assert_eq!(site_label("project.github.io"), Some("project"));
        assert_eq!(site_label("localhost"), None);
        assert_eq!(site_label("127.0.0.1"), None);
        assert!(same_site("docs.example.co.uk", "example.co.uk"));
        assert!(!same_site("a.github.io", "b.github.io"));
        assert!(!same_site("example.co.uk", "other.co.uk"));
    }

    #[test]
    fn remembers_one_engine_per_site() {
        let mut engines = SiteEngines::default();
        let engine = parse(YOUTUBE, "youtube.com").unwrap();
        assert!(engines.add(engine.clone()));
        assert!(!engines.add(engine.clone()));
        assert!(engines.knows("www.youtube.com"));
        let copy = SiteEngines::from_json(&engines.to_json()).unwrap();
        assert_eq!(copy, engines);
        assert_eq!(engines.len(), 1);
    }
}
