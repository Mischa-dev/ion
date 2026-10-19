//! Per-site user agent (`userAgent` in `[sites]`).
//!
//! Every site gets [`default_user_agent`]: the engine's user agent without
//! its `QtWebEngine/…` token, which is Chromium's own. Sites that see the
//! token treat Ion as an embedded app (Google sign-in refuses it, others
//! show "browser not supported"); without it Ion reads as the Chromium
//! version it is, and matches the client hints the engine sends
//! (`Sec-CH-UA: "Chromium";v="…"`).
//!
//! A `userAgent` value is a preset name or a full user-agent string:
//!
//! - `"default"` (or empty, or `"chrome"`): Ion's default above; useful to
//!   undo a `"*"` entry or a built-in quirk for one site.
//! - `"firefox"`, `"safari"`: a recent Firefox or Safari on this platform.
//! - `"engine"`: the engine's user agent with the `QtWebEngine/…` token.
//!
//! A few sites get a built-in value ([`QUIRKS`]) unless config sets one for
//! them. The header comes from the request interceptor ([`for_url`]); pages
//! read the same value from `navigator.userAgent` through [`script`].

use std::borrow::Cow;
use std::collections::BTreeMap;

use ion_config::Site;

use crate::rules::matching;
use crate::scripts::{RunAt, Script, World};

const FIREFOX_LINUX: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:150.0) Gecko/20100101 Firefox/150.0";
const FIREFOX_MAC: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:150.0) Gecko/20100101 Firefox/150.0";
const SAFARI: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 \
                      (KHTML, like Gecko) Version/26.0 Safari/605.1.15";

/// Sites that need another user agent than Ion's default to work, as
/// `[sites]` keys and `userAgent` values. Config entries for the same key
/// win, so `userAgent = "default"` there turns one off.
///
/// - Google sign-in rejects Chromium-based browsers it can't identify as
///   Chrome ("This browser or app may not be secure"), but signs in a
///   Firefox, which never sends Chromium's client hints.
pub const QUIRKS: &[(&str, &str)] = &[("accounts.google.com", "firefox")];

/// The user agent every page gets unless `[sites]` or [`QUIRKS`] say
/// otherwise: `engine` without its `QtWebEngine/…` token. Ion sets it as
/// the profile's own.
pub fn default_user_agent(engine: &str) -> String {
    without_qtwebengine(engine)
}

/// `sites` with the [`QUIRKS`] that no entry overrides added.
fn with_quirks(sites: &BTreeMap<String, Site>) -> Cow<'_, BTreeMap<String, Site>> {
    let normalized = |key: &str| key.trim().trim_end_matches('.').to_ascii_lowercase();
    let missing: Vec<_> = QUIRKS
        .iter()
        .filter(|(quirk, _)| {
            !sites
                .iter()
                .any(|(key, site)| normalized(key) == *quirk && site.user_agent.is_some())
        })
        .collect();
    if missing.is_empty() {
        return Cow::Borrowed(sites);
    }
    let mut sites = sites.clone();
    for (key, value) in missing {
        // An entry for the site may set other things; keep them.
        let existing = sites.keys().find(|k| normalized(k) == *key).cloned();
        let site = sites
            .entry(existing.unwrap_or_else(|| (*key).to_owned()))
            .or_default();
        site.user_agent = Some((*value).to_owned());
    }
    Cow::Owned(sites)
}

/// The user agent `value` stands for, given the engine's own `engine` user
/// agent. `None` means Ion's [`default_user_agent`].
pub fn resolve(value: &str, engine: &str) -> Option<String> {
    let value = value.trim();
    match value.to_ascii_lowercase().as_str() {
        "" | "default" | "chrome" => None,
        "engine" => Some(engine.to_owned()),
        "firefox" if cfg!(target_os = "macos") => Some(FIREFOX_MAC.to_owned()),
        "firefox" => Some(FIREFOX_LINUX.to_owned()),
        "safari" => Some(SAFARI.to_owned()),
        _ => Some(value.to_owned()),
    }
}

/// `engine` with its `QtWebEngine/x.y.z` product token removed.
fn without_qtwebengine(engine: &str) -> String {
    engine
        .split(' ')
        .filter(|token| !token.starts_with("QtWebEngine/"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether pages sent `user_agent` should keep the engine's client hints
/// (`Sec-CH-UA*`, `navigator.userAgentData`). Only the engine's own user
/// agent, with or without its token, matches them; any other value would
/// contradict them, and Firefox and Safari send none.
pub fn keeps_client_hints(user_agent: &str, engine: &str) -> bool {
    user_agent == engine || user_agent == without_qtwebengine(engine)
}

/// The user agent to send for pages at `url`: the most specific matching
/// entry (or quirk) that sets `userAgent`, or `None` for Ion's default.
pub fn for_url(sites: &BTreeMap<String, Site>, url: &str, engine: &str) -> Option<String> {
    let sites = with_quirks(sites);
    matching(&sites, url)
        .iter()
        .rev()
        .find_map(|(_, site)| site.user_agent.as_deref())
        .and_then(|value| resolve(value, engine))
}

/// A page script that makes `navigator.userAgent` (and `appVersion`, `vendor`,
/// `productSub`, `platform`) agree with the header on sites that override it.
/// `None` when no entry does.
pub fn script(sites: &BTreeMap<String, Site>, engine: &str) -> Option<Script> {
    let sites = with_quirks(sites);
    // (site, user agent or null for the engine's, keep client hints)
    let mut entries: Vec<(String, Option<String>, bool)> = sites
        .iter()
        .filter_map(|(key, site)| {
            let value = site.user_agent.as_deref()?;
            let key = key.trim().trim_end_matches('.').to_ascii_lowercase();
            let ua = resolve(value, engine);
            let hints = ua
                .as_deref()
                .is_none_or(|ua| keeps_client_hints(ua, engine));
            Some((key, ua, hints))
        })
        .collect();
    if entries.iter().all(|(_, ua, _)| ua.is_none()) {
        return None;
    }
    // Least specific first, like `rules::matching`; the page keeps the last hit.
    entries.sort_by_key(|(key, _, _)| if key == "*" { 0 } else { key.len() + 1 });
    let entries = serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_owned());
    let source = format!(
        r#"(() => {{
  const entries = {entries};
  const host = location.hostname.replace(/\.$/, "").toLowerCase();
  const covers = (key) => {{
    if (key === "*") return true;
    const k = key.startsWith("*.") ? key.slice(2) : key;
    return host === k || host.endsWith("." + k);
  }};
  let ua = null, hints = true;
  for (const [key, value, keep] of entries) if (covers(key)) [ua, hints] = [value, keep];
  if (!ua) return;
  const get = (value) => ({{ get: () => value, configurable: true, enumerable: true }});
  Object.defineProperty(Navigator.prototype, "userAgent", get(ua));
  Object.defineProperty(Navigator.prototype, "appVersion", get(ua.replace(/^Mozilla\//, "")));
  // The other identity fields, so they don't contradict the claimed browser.
  const gecko = /Firefox\//.test(ua) && !/AppleWebKit/.test(ua);
  const apple = /Version\/[\d.]+.*Safari\//.test(ua) && !/Chrome|Chromium/.test(ua);
  Object.defineProperty(Navigator.prototype, "vendor",
    get(gecko ? "" : apple ? "Apple Computer, Inc." : "Google Inc."));
  Object.defineProperty(Navigator.prototype, "productSub", get(gecko ? "20100101" : "20030107"));
  const platform = /iPad/.test(ua) ? "iPad" : /iPhone/.test(ua) ? "iPhone"
    : /Macintosh/.test(ua) ? "MacIntel" : /Windows/.test(ua) ? "Win32"
    : /Android/.test(ua) ? "Linux armv8l" : /Linux/.test(ua) ? "Linux x86_64" : null;
  if (platform) Object.defineProperty(Navigator.prototype, "platform", get(platform));
  // Gone, not undefined, so `"userAgentData" in navigator` checks agree too.
  if (!hints) delete Navigator.prototype.userAgentData;
}})();
"#
    );
    Some(Script {
        name: "ion-user-agent".to_owned(),
        source,
        run_at: RunAt::DocumentStart,
        world: World::Main,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENGINE: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
                          QtWebEngine/6.11.2 Chrome/134.0.6998.208 Safari/537.36";

    fn sites(entries: &[(&str, &str)]) -> BTreeMap<String, Site> {
        entries
            .iter()
            .map(|(k, ua)| {
                (
                    (*k).to_owned(),
                    Site {
                        user_agent: Some((*ua).to_owned()),
                        ..Site::default()
                    },
                )
            })
            .collect()
    }

    #[test]
    fn default_drops_the_qtwebengine_token() {
        assert_eq!(
            default_user_agent(ENGINE),
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/134.0.6998.208 Safari/537.36"
        );
        assert!(keeps_client_hints(&default_user_agent(ENGINE), ENGINE));
        assert!(keeps_client_hints(ENGINE, ENGINE));
    }

    #[test]
    fn google_sign_in_gets_firefox_unless_config_says_otherwise() {
        let none = BTreeMap::new();
        let google = for_url(&none, "https://accounts.google.com/v3/signin", ENGINE).unwrap();
        assert!(google.contains("Firefox/"), "{google}");
        assert_eq!(for_url(&none, "https://www.google.com/", ENGINE), None);
        assert!(
            script(&none, ENGINE)
                .unwrap()
                .source
                .contains("accounts.google.com")
        );
        // A config entry for the site wins, even with different spelling.
        let own = sites(&[("Accounts.Google.com.", "default")]);
        assert_eq!(for_url(&own, "https://accounts.google.com/", ENGINE), None);
        // One that sets something else keeps the quirk and its own setting.
        let mut js = BTreeMap::new();
        js.insert(
            "accounts.google.com".to_owned(),
            Site {
                javascript: Some(false),
                ..Site::default()
            },
        );
        let merged = with_quirks(&js);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged["accounts.google.com"].javascript, Some(false));
        assert!(for_url(&js, "https://accounts.google.com/", ENGINE).is_some());
    }

    #[test]
    fn presets_and_custom_strings() {
        assert_eq!(resolve("Chrome", ENGINE), None);
        assert_eq!(resolve("engine", ENGINE).as_deref(), Some(ENGINE));
        assert!(resolve("firefox", ENGINE).unwrap().contains("Firefox/"));
        assert!(resolve("safari", ENGINE).unwrap().contains("Version/"));
        assert_eq!(resolve(" default ", ENGINE), None);
        assert_eq!(resolve("", ENGINE), None);
        assert_eq!(resolve("MyAgent/1.0", ENGINE).unwrap(), "MyAgent/1.0");
        assert!(!keeps_client_hints(
            &resolve("firefox", ENGINE).unwrap(),
            ENGINE
        ));
        assert!(!keeps_client_hints(
            &resolve("safari", ENGINE).unwrap(),
            ENGINE
        ));
        let old_chrome = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
                          Chrome/99.0.0.0 Safari/537.36";
        assert!(!keeps_client_hints(old_chrome, ENGINE));
    }

    #[test]
    fn most_specific_entry_wins() {
        let sites = sites(&[
            ("*", "engine"),
            ("example.com", "default"),
            ("a.org", "x/1"),
        ]);
        assert!(
            for_url(&sites, "https://other.net/", ENGINE)
                .unwrap()
                .contains("QtWebEngine")
        );
        assert_eq!(for_url(&sites, "https://www.example.com/", ENGINE), None);
        assert_eq!(
            for_url(&sites, "https://a.org/x", ENGINE).as_deref(),
            Some("x/1")
        );
        assert_eq!(for_url(&BTreeMap::new(), "https://a.org/", ENGINE), None);
    }

    #[test]
    fn script_only_when_something_is_overridden() {
        let no_quirks = sites(&[("accounts.google.com", "default")]);
        assert!(script(&no_quirks, ENGINE).is_none());
        let s = script(&sites(&[("a.org", "x/\"1</script>")]), ENGINE).unwrap();
        assert_eq!(s.world, World::Main);
        assert!(s.source.contains(r#"["a.org","x/\"1</script>",false]"#));
    }

    #[test]
    fn script_keeps_the_other_identity_fields_consistent() {
        let s = script(&sites(&[("a.org", "safari")]), ENGINE).unwrap();
        for field in ["\"vendor\"", "\"productSub\"", "\"platform\""] {
            assert!(s.source.contains(field), "{field}");
        }
    }
}
