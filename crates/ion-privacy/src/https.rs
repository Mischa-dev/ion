//! HTTPS-first: pages asked for over plain http load over https instead.
//!
//! The request interceptor asks [`Https::upgrade`] for every top-level
//! navigation. When the https version then fails to load, the tab asks
//! [`Https::fallback`]: in [`HttpsMode::Upgrade`] it goes back to http (and
//! remembers the site for the session), in [`HttpsMode::Strict`] it shows a
//! warning with a button that calls [`Https::allow_http`].

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// How hard Ion insists on https.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpsMode {
    /// Load pages as asked.
    Off,
    /// Try https first and fall back to http quietly (Chrome and Brave's
    /// default).
    Upgrade,
    /// Try https first and warn before loading http (Firefox's HTTPS-Only).
    Strict,
}

impl HttpsMode {
    /// From the config value: "off", "upgrade" or "strict".
    pub fn parse(value: &str) -> Self {
        match value {
            "off" => Self::Off,
            "strict" => Self::Strict,
            _ => Self::Upgrade,
        }
    }
}

/// A server that sends https visitors back to http answers within this long;
/// seeing its http address again that soon means a loop, not a click.
const LOOP_WINDOW: Duration = Duration::from_secs(10);

/// What happened to a top-level request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Upgrade {
    /// Load as asked.
    Keep,
    /// Load this https address instead.
    To(String),
}

/// The session's upgrade state: sites that only work over http, and sites
/// upgraded a moment ago.
#[derive(Debug, Default)]
pub struct Https {
    /// Hosts loaded over http for the rest of the session.
    allowed: HashSet<String>,
    /// Hosts Ion upgraded, and when.
    upgraded: HashMap<String, Instant>,
}

impl Https {
    /// What to do with a top-level navigation to `url`.
    pub fn upgrade(&mut self, url: &str, mode: HttpsMode, now: Instant) -> Upgrade {
        if mode == HttpsMode::Off {
            return Upgrade::Keep;
        }
        let Ok(mut parsed) = url::Url::parse(url) else {
            return Upgrade::Keep;
        };
        if parsed.scheme() != "http" {
            return Upgrade::Keep;
        }
        let Some(host) = parsed.host_str().map(str::to_ascii_lowercase) else {
            return Upgrade::Keep;
        };
        if !is_public_host(&host) || self.allowed.contains(&host) {
            return Upgrade::Keep;
        }
        // The https site sent us back here: it only works over http.
        if let Some(when) = self.upgraded.get(&host)
            && now.duration_since(*when) < LOOP_WINDOW
        {
            self.upgraded.remove(&host);
            if mode == HttpsMode::Upgrade {
                self.allowed.insert(host);
                return Upgrade::Keep;
            }
        }
        // A non-default port (http://site:8080) has no https twin to guess.
        if parsed.port().is_some() || parsed.set_scheme("https").is_err() {
            return Upgrade::Keep;
        }
        self.upgraded.insert(host, now);
        Upgrade::To(parsed.into())
    }

    /// After the https page at `url` failed to load: the http address to try
    /// instead, if Ion is the one that upgraded it. In `Upgrade` mode the
    /// site is remembered as http-only; in `Strict` mode only
    /// [`Https::allow_http`] does that.
    pub fn fallback(&mut self, url: &str, mode: HttpsMode) -> Option<String> {
        let mut parsed = url::Url::parse(url).ok()?;
        if parsed.scheme() != "https" {
            return None;
        }
        let host = parsed.host_str()?.to_ascii_lowercase();
        self.upgraded.remove(&host)?;
        parsed.set_scheme("http").ok()?;
        if mode == HttpsMode::Upgrade {
            self.allowed.insert(host);
        }
        Some(parsed.into())
    }

    /// Load `url`'s site over http for the rest of the session.
    pub fn allow_http(&mut self, url: &str) {
        if let Some(host) = url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
        {
            self.upgraded.remove(&host);
            self.allowed.insert(host);
        }
    }

    /// Whether `url`'s site was let through over http this session.
    pub fn is_allowed(&self, url: &str) -> bool {
        url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
            .is_some_and(|h| self.allowed.contains(&h))
    }
}

/// Whether `host` is on the public internet, where https is expected:
/// not an IP address, `localhost`, a single-label intranet name or a local
/// name such as `printer.local`, `nas.lan` or `router.home.arpa`.
fn is_public_host(host: &str) -> bool {
    let host = host.trim_end_matches('.');
    if host.starts_with('[') || host.parse::<std::net::Ipv4Addr>().is_ok() {
        return false;
    }
    if !host.contains('.') {
        return false;
    }
    const LOCAL: &[&str] = &[
        ".local",
        ".lan",
        ".home",
        ".internal",
        ".intranet",
        ".corp",
        ".localhost",
        ".test",
        ".home.arpa",
        ".onion",
        ".i2p",
    ];
    !LOCAL.iter().any(|suffix| host.ends_with(suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to(url: &str) -> Upgrade {
        Upgrade::To(url.to_owned())
    }

    #[test]
    fn plain_http_pages_go_https() {
        let mut https = Https::default();
        let now = Instant::now();
        assert_eq!(
            https.upgrade("http://example.com/a?b=1", HttpsMode::Upgrade, now),
            to("https://example.com/a?b=1")
        );
        assert_eq!(
            https.upgrade("https://example.com/", HttpsMode::Upgrade, now),
            Upgrade::Keep
        );
        assert_eq!(
            https.upgrade("http://example.com/", HttpsMode::Off, now),
            Upgrade::Keep
        );
        for local in [
            "http://localhost:8000/",
            "http://127.0.0.1/",
            "http://[::1]/",
            "http://router/",
            "http://printer.local/",
            "http://nas.home.arpa/",
            "http://example.com:8080/",
        ] {
            assert_eq!(
                https.upgrade(local, HttpsMode::Strict, now),
                Upgrade::Keep,
                "{local}"
            );
        }
    }

    #[test]
    fn failed_upgrades_fall_back_and_stick_for_the_session() {
        let mut https = Https::default();
        let now = Instant::now();
        https.upgrade("http://old.example/", HttpsMode::Upgrade, now);
        // A site Ion didn't upgrade has nothing to fall back to.
        assert_eq!(
            https.fallback("https://other.example/", HttpsMode::Upgrade),
            None
        );
        assert_eq!(
            https
                .fallback("https://old.example/", HttpsMode::Upgrade)
                .as_deref(),
            Some("http://old.example/")
        );
        assert!(https.is_allowed("http://old.example/x"));
        let later = now + Duration::from_secs(60);
        assert_eq!(
            https.upgrade("http://old.example/x", HttpsMode::Upgrade, later),
            Upgrade::Keep
        );
    }

    #[test]
    fn strict_mode_waits_for_permission() {
        let mut https = Https::default();
        let now = Instant::now();
        https.upgrade("http://old.example/", HttpsMode::Strict, now);
        assert_eq!(
            https
                .fallback("https://old.example/", HttpsMode::Strict)
                .as_deref(),
            Some("http://old.example/")
        );
        assert!(!https.is_allowed("http://old.example/"));
        https.allow_http("http://old.example/");
        assert_eq!(
            https.upgrade("http://old.example/", HttpsMode::Strict, now),
            Upgrade::Keep
        );
    }

    #[test]
    fn a_redirect_back_to_http_ends_the_loop() {
        let mut https = Https::default();
        let now = Instant::now();
        assert!(matches!(
            https.upgrade("http://loop.example/", HttpsMode::Upgrade, now),
            Upgrade::To(_)
        ));
        // The https site redirected to http a moment later.
        let soon = now + Duration::from_secs(1);
        assert_eq!(
            https.upgrade("http://loop.example/", HttpsMode::Upgrade, soon),
            Upgrade::Keep
        );
        assert!(https.is_allowed("http://loop.example/"));
        // Much later, a fresh visit is upgraded again.
        let mut fresh = Https::default();
        fresh.upgrade("http://slow.example/", HttpsMode::Upgrade, now);
        let late = now + Duration::from_secs(120);
        assert!(matches!(
            fresh.upgrade("http://slow.example/", HttpsMode::Upgrade, late),
            Upgrade::To(_)
        ));
    }
}
