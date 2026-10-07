//! The per-site switch: sites where blocking is turned off.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

/// The site a URL belongs to, as shown to people and used for counters and
/// the per-site switch: its lower-cased host without a leading `www.`.
/// `None` for URLs without a host (`about:blank`, `file:`, `data:`).
pub fn site_of(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    let site = normalize(parsed.host_str()?);
    (!site.is_empty()).then_some(site)
}

/// Lower-case, no trailing dot, no leading `www.`, unless that would leave a
/// public suffix: `www.com` and `www.co.uk` stay as they are, so switching
/// one off doesn't switch off every `.com` or `.co.uk` site.
fn normalize(host: &str) -> String {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    match host.strip_prefix("www.") {
        Some(rest) if !is_public_suffix(rest) => rest.to_owned(),
        _ => host,
    }
}

/// Whether `domain` is a suffix listed in the public suffix list, like `com`,
/// `co.uk` or `github.io`. Hosts the list doesn't know, such as `localhost`
/// or an intranet name, are not.
fn is_public_suffix(domain: &str) -> bool {
    psl::suffix(domain.as_bytes())
        .is_some_and(|s| s.is_known() && s.as_bytes() == domain.as_bytes())
}

/// Sites where blocking is off. Turning a site off also covers its subdomains,
/// so switching off `example.com` lets `shop.example.com` through too.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SiteSettings {
    disabled: BTreeSet<String>,
}

impl SiteSettings {
    /// Parse the saved form: one site per line, `#` starts a comment.
    pub fn parse(text: &str) -> Self {
        let disabled = text
            .lines()
            .map(|line| line.split('#').next().unwrap_or("").trim())
            .filter(|line| !line.is_empty())
            .map(normalize)
            .filter(|site| !is_public_suffix(site))
            .collect();
        Self { disabled }
    }

    pub fn serialize(&self) -> String {
        let mut out =
            String::from("# Sites where Ion's ad blocker is switched off, one per line.\n");
        for site in &self.disabled {
            out.push_str(site);
            out.push('\n');
        }
        out
    }

    /// Load from `path`; a missing file means no exceptions.
    pub fn load(path: &Path) -> io::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(Self::parse(&text)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        crate::store::write_atomic(path, self.serialize().as_bytes())
    }

    /// Where the switches are saved by default.
    pub fn default_path() -> Option<PathBuf> {
        Some(
            dirs::data_dir()?
                .join("ion")
                .join("adblock")
                .join("disabled-sites.txt"),
        )
    }

    /// Whether blocking applies on `site` (a host as returned by [`site_of`]).
    pub fn is_enabled(&self, site: &str) -> bool {
        !suffixes(site).any(|s| self.disabled.contains(s))
    }

    /// Switch blocking on or off for `site`. Switching a subdomain back on
    /// also clears a switch on its parent domain, since that one covered it.
    /// Public suffixes like `github.io` can't be switched off, since that
    /// would switch off every site under them; returns false for those.
    pub fn set_enabled(&mut self, site: &str, enabled: bool) -> bool {
        let site = normalize(site);
        if !enabled && is_public_suffix(&site) {
            return false;
        }
        if enabled {
            let covering: Vec<String> = suffixes(&site)
                .filter(|s| self.disabled.contains(*s))
                .map(str::to_owned)
                .collect();
            for s in covering {
                self.disabled.remove(&s);
            }
        } else {
            self.disabled.insert(site);
        }
        true
    }

    pub fn disabled_sites(&self) -> impl Iterator<Item = &str> {
        self.disabled.iter().map(String::as_str)
    }
}

/// `a.b.example.com`, `b.example.com`, `example.com`, `com`.
fn suffixes(host: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(Some(host), |h| h.split_once('.').map(|(_, rest)| rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_of_strips_www_and_case() {
        assert_eq!(
            site_of("https://WWW.Example.com/a?b").as_deref(),
            Some("example.com")
        );
        assert_eq!(
            site_of("http://shop.example.com:8080/").as_deref(),
            Some("shop.example.com")
        );
        assert_eq!(
            site_of("https://example.com./").as_deref(),
            Some("example.com")
        );
        // Never collapse to a bare TLD.
        assert_eq!(site_of("https://www.com/").as_deref(), Some("www.com"));
        assert_eq!(site_of("https://www.co.uk/").as_deref(), Some("www.co.uk"));
        assert_eq!(
            site_of("http://www.localhost/").as_deref(),
            Some("localhost")
        );
        assert_eq!(
            site_of("https://www.bbc.co.uk/").as_deref(),
            Some("bbc.co.uk")
        );
        assert_eq!(site_of("about:blank"), None);
        assert_eq!(site_of("file:///etc/hosts"), None);
        assert_eq!(site_of("garbage"), None);
    }

    #[test]
    fn disabling_covers_subdomains() {
        let mut s = SiteSettings::default();
        s.set_enabled("www.example.com", false);
        assert!(!s.is_enabled("example.com"));
        assert!(!s.is_enabled("shop.example.com"));
        assert!(s.is_enabled("example.org"));
        assert!(s.is_enabled("notexample.com"));
    }

    #[test]
    fn enabling_a_subdomain_clears_the_parent() {
        let mut s = SiteSettings::default();
        s.set_enabled("example.com", false);
        s.set_enabled("shop.example.com", true);
        assert!(s.is_enabled("shop.example.com"));
        assert!(s.is_enabled("example.com"));
    }

    #[test]
    fn round_trips_through_text() {
        let mut s = SiteSettings::default();
        s.set_enabled("b.example", false);
        s.set_enabled("a.example", false);
        let text = s.serialize();
        assert_eq!(SiteSettings::parse(&text), s);
        assert_eq!(
            s.disabled_sites().collect::<Vec<_>>(),
            ["a.example", "b.example"]
        );
    }

    #[test]
    fn parse_ignores_comments_and_blanks() {
        let s = SiteSettings::parse("# hi\n\n  WWW.Example.com  # bank\n");
        assert_eq!(s.disabled_sites().collect::<Vec<_>>(), ["example.com"]);
    }

    #[test]
    fn load_and_save() {
        let dir = crate::store::tests::temp_dir("sites");
        let path = dir.join("nested").join("sites.txt");
        assert_eq!(SiteSettings::load(&path).unwrap(), SiteSettings::default());
        let mut s = SiteSettings::default();
        s.set_enabled("example.com", false);
        s.save(&path).unwrap();
        assert_eq!(SiteSettings::load(&path).unwrap(), s);
    }

    #[test]
    fn public_suffixes_cannot_be_switched_off() {
        let mut s = SiteSettings::default();
        assert!(!s.set_enabled("github.io", false));
        assert!(!s.set_enabled("co.uk", false));
        assert!(s.is_enabled("someone.github.io"));
        assert!(s.set_enabled("someone.github.io", false));
        assert!(!s.is_enabled("someone.github.io"));
        assert!(s.is_enabled("other.github.io"));
        // Local and intranet hosts are not public suffixes.
        assert!(s.set_enabled("localhost", false));
        assert!(!s.is_enabled("localhost"));
        assert!(s.set_enabled("intranet", false));
        // Nor sneaked in through the saved file.
        let parsed = SiteSettings::parse("github.io\nexample.com\n");
        assert_eq!(parsed.disabled_sites().collect::<Vec<_>>(), ["example.com"]);
    }
}
