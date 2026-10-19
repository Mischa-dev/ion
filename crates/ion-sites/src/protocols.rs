//! Links that open other apps (`mailto:`, `zoommtg:`, `slack:`…).
//!
//! The engine hands any scheme it doesn't load itself to the desktop
//! (`xdg-open`, Launch Services). Ion asks first, like other browsers, so a
//! page can't start apps behind the person's back; "always allow" is kept per
//! site and scheme in [`Approvals`]. Sites can also ask to become the handler
//! for a scheme (`navigator.registerProtocolHandler`, say Gmail for
//! `mailto:`); [`handler_prompt`] words that question.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::rules::host_of;

/// Schemes the engine loads itself; every other one goes to another app.
const WEB_SCHEMES: &[&str] = &[
    "http",
    "https",
    "file",
    "data",
    "blob",
    "about",
    "javascript",
    "filesystem",
    "chrome",
    "chrome-extension",
    "chrome-error",
    "chrome-untrusted",
    "devtools",
    "view-source",
    "qrc",
    "ws",
    "wss",
    "ion",
];

/// The scheme of `url`, lowercase, or `None` without one.
pub fn scheme(url: &str) -> Option<String> {
    let (scheme, _) = url.split_once(':')?;
    let valid = scheme
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    valid.then(|| scheme.to_ascii_lowercase())
}

/// Whether a navigation to `url` would leave the browser for another app.
pub fn is_external(url: &str) -> bool {
    scheme(url).is_some_and(|s| !WEB_SCHEMES.contains(&s.as_str()))
}

/// What people call the app that opens `scheme`, for prompts.
pub fn app_name(scheme: &str) -> String {
    let name = match scheme {
        "mailto" => "your email app",
        "tel" | "callto" => "your phone app",
        "sms" => "your messages app",
        "webcal" | "webcals" => "your calendar",
        "magnet" => "your torrent app",
        "zoommtg" | "zoomus" | "zoomphonecall" => "Zoom",
        "msteams" => "Microsoft Teams",
        "slack" => "Slack",
        "discord" => "Discord",
        "spotify" => "Spotify",
        "steam" => "Steam",
        "tg" => "Telegram",
        "signal" | "sgnl" => "Signal",
        "element" | "io.element.desktop" => "Element",
        "vscode" | "vscode-insiders" => "VS Code",
        "obsidian" => "Obsidian",
        "figma" => "Figma",
        "notion" => "Notion",
        "linear" => "Linear",
        "ssh" => "your SSH client",
        "git" => "your Git client",
        _ => return format!("an app for {scheme}: links"),
    };
    name.to_owned()
}

/// The site a page at `origin` is named by in prompts.
fn site_name(origin: &str) -> String {
    host_of(origin)
        .map(|h| h.strip_prefix("www.").map(str::to_owned).unwrap_or(h))
        .unwrap_or_else(|| "This page".to_owned())
}

/// `[question, always-allow label]` for a page at `origin` opening `url`.
pub fn open_prompt(origin: &str, url: &str) -> (String, String) {
    let scheme = scheme(url).unwrap_or_default();
    let site = site_name(origin);
    (
        format!("{site} wants to open {}.", app_name(&scheme)),
        format!("Always let {site} open {scheme}: links"),
    )
}

/// The question for `origin` asking to handle `scheme` links itself.
pub fn handler_prompt(origin: &str, scheme: &str) -> String {
    let site = site_name(origin);
    let what = match scheme.to_ascii_lowercase().as_str() {
        "mailto" => "email links".to_owned(),
        "webcal" => "calendar links".to_owned(),
        "tel" => "phone number links".to_owned(),
        "magnet" => "magnet links".to_owned(),
        other => format!("{} links", other.strip_prefix("web+").unwrap_or(other)),
    };
    format!("Open {what} with {site}? It will open them instead of another app.")
}

/// The "always allow" answers (which sites may open which schemes without
/// asking) and the sites people let handle a scheme themselves. Saved as
/// JSON: `{ "allowed": { "https://site": ["zoommtg"] },
/// "handlers": { "mailto": "https://mail.google.com" } }`.
#[derive(Debug, Default)]
pub struct Approvals {
    path: Option<PathBuf>,
    allowed: BTreeMap<String, BTreeSet<String>>,
    handlers: BTreeMap<String, String>,
}

impl Approvals {
    /// Approvals kept in `path`; a missing or unreadable file means none.
    pub fn load(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let json: serde_json::Value = std::fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        let field = |name: &str| json.get(name).cloned().unwrap_or_default();
        Self {
            path: Some(path),
            allowed: serde_json::from_value(field("allowed")).unwrap_or_default(),
            handlers: serde_json::from_value(field("handlers")).unwrap_or_default(),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Whether pages at `origin` may open `url` without asking.
    pub fn allows(&self, origin: &str, url: &str) -> bool {
        let (Some(site), Some(scheme)) = (site_key(origin), scheme(url)) else {
            return false;
        };
        self.allowed.get(&site).is_some_and(|s| s.contains(&scheme))
    }

    /// Let pages at `origin` open links like `url` from now on, and save.
    pub fn allow(&mut self, origin: &str, url: &str) -> std::io::Result<()> {
        let (Some(site), Some(scheme)) = (site_key(origin), scheme(url)) else {
            return Ok(());
        };
        self.allowed.entry(site).or_default().insert(scheme);
        self.save()
    }

    /// Record that the site at `origin` now handles `scheme` links (the
    /// engine sends them there instead of to another app), and save.
    pub fn set_handler(&mut self, scheme: &str, origin: &str) -> std::io::Result<()> {
        let Some(site) = site_key(origin) else {
            return Ok(());
        };
        self.handlers.insert(scheme.to_ascii_lowercase(), site);
        self.save()
    }

    /// Whether a site handles links like `url`, so the engine opens them
    /// itself instead of handing them to another app.
    pub fn has_handler(&self, url: &str) -> bool {
        scheme(url).is_some_and(|s| self.handlers.contains_key(&s))
    }

    /// Forget every approval for pages at `origin`, and the schemes it
    /// handles, and save.
    pub fn forget(&mut self, origin: &str) -> std::io::Result<()> {
        let Some(site) = site_key(origin) else {
            return Ok(());
        };
        let before = self.handlers.len();
        self.handlers.retain(|_, handler| *handler != site);
        if self.allowed.remove(&site).is_some() || self.handlers.len() != before {
            return self.save();
        }
        Ok(())
    }

    fn save(&self) -> std::io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::json!({
            "allowed": self.allowed,
            "handlers": self.handlers,
        });
        let json = serde_json::to_string_pretty(&json).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }
}

/// `scheme://host[:port]` for an origin or any URL on it, or `None` for
/// pages without a host.
fn site_key(origin: &str) -> Option<String> {
    let parsed = url::Url::parse(origin).ok()?;
    let host = parsed.host_str().filter(|h| !h.is_empty())?;
    Some(match parsed.port() {
        Some(port) => format!("{}://{host}:{port}", parsed.scheme()),
        None => format!("{}://{host}", parsed.scheme()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_schemes_stay_in_the_browser() {
        for url in [
            "https://a.com/",
            "HTTP://a.com",
            "about:blank",
            "blob:https://a.com/x",
            "view-source:https://a.com",
            "chrome://gpu",
        ] {
            assert!(!is_external(url), "{url}");
        }
        for url in [
            "mailto:a@b.c",
            "zoommtg://zoom.us/join?confno=1",
            "tel:+1555",
            "web+foo:x",
        ] {
            assert!(is_external(url), "{url}");
        }
        assert!(!is_external("no scheme here"));
        assert_eq!(scheme("1abc:x"), None);
    }

    #[test]
    fn prompts_name_the_site_and_app() {
        let (question, always) = open_prompt("https://www.zoom.us/j/1", "zoommtg://zoom.us/join");
        assert_eq!(question, "zoom.us wants to open Zoom.");
        assert_eq!(always, "Always let zoom.us open zoommtg: links");
        assert_eq!(
            open_prompt("https://a.com", "foo:bar").0,
            "a.com wants to open an app for foo: links."
        );
        assert!(
            handler_prompt("https://mail.google.com", "mailto")
                .starts_with("Open email links with mail.google.com?")
        );
        assert!(handler_prompt("https://x.org", "web+coffee").starts_with("Open coffee links"));
    }

    #[test]
    fn approvals_are_per_site_and_scheme_and_saved() {
        let dir = std::env::temp_dir().join(format!("ion-protocols-{}", std::process::id()));
        let path = dir.join("protocols.json");
        let _ = std::fs::remove_dir_all(&dir);
        let mut approvals = Approvals::load(&path);
        assert!(!approvals.allows("https://zoom.us", "zoommtg://x"));
        approvals
            .allow("https://zoom.us/j/1", "zoommtg://x")
            .unwrap();
        assert!(approvals.allows("https://zoom.us", "ZOOMMTG://y"));
        assert!(!approvals.allows("https://zoom.us", "mailto:a@b"));
        assert!(!approvals.allows("http://zoom.us", "zoommtg://x"));
        let reloaded = Approvals::load(&path);
        assert!(reloaded.allows("https://zoom.us/", "zoommtg://x"));
        let mut reloaded = reloaded;
        reloaded.forget("https://zoom.us").unwrap();
        assert!(!Approvals::load(&path).allows("https://zoom.us", "zoommtg://x"));
        // Sites that handle a scheme themselves.
        approvals
            .set_handler("MailTo", "https://mail.google.com/mail/u/0")
            .unwrap();
        assert!(Approvals::load(&path).has_handler("mailto:a@b.c"));
        assert!(!approvals.has_handler("tel:1"));
        approvals.forget("https://mail.google.com").unwrap();
        assert!(!Approvals::load(&path).has_handler("mailto:a@b.c"));
        // Pages without a host can't be approved.
        approvals.allow("file:///tmp/x.html", "mailto:a").unwrap();
        assert!(!approvals.allows("file:///tmp/x.html", "mailto:a"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
