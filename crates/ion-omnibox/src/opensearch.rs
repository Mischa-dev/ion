//! Site search engines found on the sites you visit.
//!
//! Many sites describe their own search in an OpenSearch file
//! (`<link rel="search" type="application/opensearchdescription+xml">`).
//! Ion reads it once per site and remembers the search, so typing the site's
//! name and pressing Tab searches it, the way Chrome's site search works.
//! Each found engine becomes a bang (`!youtube`, `!docs.rs`) alongside the
//! built-in ones, never replacing a bang that already exists.

use serde::{Deserialize, Serialize};

/// Sites remembered at most; the oldest are dropped first.
pub const MAX_ENGINES: usize = 300;

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

/// The site's search from an OpenSearch description `xml` fetched for a page
/// on `page_host`. The results page has to be on the same site.
pub fn parse(xml: &str, page_host: &str) -> Option<SiteEngine> {
    let page_host = page_host.trim_start_matches("www.").to_lowercase();
    let name = element_text(xml, "ShortName")
        .map(|name| decode(name.trim()))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| page_host.clone());

    let template = tags(xml, "Url").find_map(|tag| {
        let kind = attribute(tag, "type").unwrap_or("text/html");
        if kind != "text/html" {
            return None;
        }
        let template = decode(attribute(tag, "template")?);
        // Only the query; optional parameters ({startPage?} and the like) go.
        if !template.contains("{searchTerms}") {
            return None;
        }
        let template = template.replace("{searchTerms}", "\u{0}");
        let template = strip_optional_parameters(&template).replace('\u{0}', "{}");
        Some(template)
    })?;

    let url = url::Url::parse(&template.replace("{}", "q")).ok()?;
    if !matches!(url.scheme(), "https" | "http") {
        return None;
    }
    let host = url.host_str()?.trim_start_matches("www.").to_lowercase();
    if !same_site(&host, &page_host) {
        return None;
    }
    Some(SiteEngine {
        host: page_host,
        name,
        template,
    })
}

/// Whether two hosts belong to one site: equal, or one a subdomain of the
/// other's registrable part.
fn same_site(a: &str, b: &str) -> bool {
    a == b || registrable_domain(a).is_some_and(|base| registrable_domain(b) == Some(base))
}

/// Drop `name={param}` / `name={param?}` pairs left in a template.
fn strip_optional_parameters(template: &str) -> String {
    let Some((base, query)) = template.split_once('?') else {
        return template.to_owned();
    };
    let kept: Vec<&str> = query
        .split('&')
        .filter(|pair| !pair.contains('{'))
        .collect();
    if kept.is_empty() {
        base.to_owned()
    } else {
        format!("{base}?{}", kept.join("&"))
    }
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
