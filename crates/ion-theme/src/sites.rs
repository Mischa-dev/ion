//! Per-site theming (`[theme.sites."<site>"]` in the config): darkening on or
//! off, and extra CSS. A site also covers its subdomains, and the most specific
//! entry wins.

use std::collections::BTreeMap;

/// What one site changes about theming.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SiteTheme {
    /// Darken the site under a dark theme (`true`) or never (`false`); `None`
    /// follows `theme.pages`.
    pub darken: Option<bool>,
    /// CSS added to the site's pages.
    pub css: String,
}

/// The host of an http(s) URL, lower-cased and without a trailing dot.
fn host_of(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    let host = parsed
        .host_str()?
        .trim_end_matches('.')
        .to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

/// The entries that apply to `url`, least specific first: for
/// `https://docs.github.com/` that is `github.com`, then `docs.github.com`.
pub fn matching<'a, T>(sites: &'a BTreeMap<String, T>, url: &str) -> Vec<&'a T> {
    let Some(host) = host_of(url) else {
        return Vec::new();
    };
    let suffixes = host
        .char_indices()
        .filter(|&(_, c)| c == '.')
        .map(|(i, _)| &host[i + 1..]);
    let mut found: Vec<&T> = [host.as_str()]
        .into_iter()
        .chain(suffixes)
        .filter_map(|suffix| sites.get(suffix))
        .collect();
    // The suffixes went from most to least specific; flip that.
    found.reverse();
    found
}

/// Whether `url` is darkened: under a dark theme, the most specific site
/// setting decides, else `default` (from `theme.pages`).
pub fn darken(sites: &BTreeMap<String, SiteTheme>, url: &str, dark: bool, default: bool) -> bool {
    dark && matching(sites, url)
        .iter()
        .rev()
        .find_map(|s| s.darken)
        .unwrap_or(default)
}

/// A user script that adds each site's CSS to its pages, or an empty string
/// when no site has any. It matches hosts itself, so one script serves every
/// page, and running it again replaces the CSS it added before.
pub fn css_script(sites: &BTreeMap<String, SiteTheme>) -> String {
    let css: BTreeMap<String, &str> = sites
        .iter()
        .filter(|(_, s)| !s.css.trim().is_empty())
        .map(|(site, s)| {
            (
                site.trim_end_matches('.').to_ascii_lowercase(),
                s.css.as_str(),
            )
        })
        .collect();
    if css.is_empty() {
        return String::new();
    }
    let map = serde_json::to_string(&css).expect("strings always serialize");
    format!(
        r#"(function () {{
  const css = {map};
  const host = location.hostname.toLowerCase().replace(/\.$/, "");
  const parts = host.split(".");
  let text = "";
  for (let i = parts.length - 1; i >= 0; i--) {{
    const site = parts.slice(i).join(".");
    if (Object.hasOwn(css, site)) text += css[site] + "\n";
  }}
  // {STATE} lives in Ion's isolated world, so it only ever holds the style
  // element this script made, never one of the page's.
  const state = (window.{STATE} ??= {{ style: null, waiting: false }});
  state.text = text;
  const apply = () => {{
    state.waiting = false;
    if (!state.text) {{
      state.style?.remove();
      state.style = null;
      return;
    }}
    if (!state.style?.isConnected) {{
      state.style = document.createElement("style");
      (document.head || document.documentElement).appendChild(state.style);
    }}
    state.style.textContent = state.text;
  }};
  if (document.documentElement) apply();
  else if (!state.waiting) {{
    // Applies whatever text is current by then, so a later run wins.
    state.waiting = true;
    document.addEventListener("DOMContentLoaded", apply, {{ once: true }});
  }}
}})();"#
    )
}

/// A script that takes away CSS added by [`css_script`], including CSS still
/// waiting for the document to load.
pub fn clear_css_script() -> String {
    format!(
        "if (window.{STATE}) {{ window.{STATE}.text = \"\"; \
         window.{STATE}.style?.remove(); window.{STATE}.style = null; }}"
    )
}

/// The isolated-world global [`css_script`] keeps its state in.
const STATE: &str = "__ionSiteCss";

#[cfg(test)]
mod tests {
    use super::*;

    fn sites(entries: &[(&str, Option<bool>, &str)]) -> BTreeMap<String, SiteTheme> {
        entries
            .iter()
            .map(|&(site, darken, css)| {
                let theme = SiteTheme {
                    darken,
                    css: css.into(),
                };
                (site.to_owned(), theme)
            })
            .collect()
    }

    #[test]
    fn hosts() {
        assert_eq!(
            host_of("https://Example.com./a?b#c").as_deref(),
            Some("example.com")
        );
        assert_eq!(
            host_of("http://user@example.com:8080/").as_deref(),
            Some("example.com")
        );
        assert_eq!(host_of("http://[::1]:8080/").as_deref(), Some("[::1]"));
        assert_eq!(host_of("about:blank"), None);
        assert_eq!(host_of("file:///tmp/x.html"), None);
    }

    #[test]
    fn matches_site_and_subdomains_least_specific_first() {
        let s = sites(&[
            ("github.com", Some(true), "a"),
            ("docs.github.com", None, "b"),
            ("hub.com", Some(false), "c"),
        ]);
        let css = |url| {
            matching(&s, url)
                .iter()
                .map(|t| t.css.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(css("https://docs.github.com/x"), ["a", "b"]);
        assert_eq!(css("https://www.github.com/"), ["a"]);
        // Only whole labels match.
        assert!(css("https://notgithub.com/").is_empty());
    }

    #[test]
    fn most_specific_darken_wins_and_only_when_dark() {
        let s = sites(&[
            ("example.com", Some(false), ""),
            ("dark.example.com", Some(true), ""),
            ("plain.example.com", None, ""),
        ]);
        assert!(!darken(&s, "https://example.com/", true, true));
        assert!(darken(&s, "https://dark.example.com/", true, false));
        // No setting of its own: the parent's applies.
        assert!(!darken(&s, "https://plain.example.com/", true, true));
        assert!(darken(&s, "https://other.org/", true, true));
        assert!(!darken(&s, "https://dark.example.com/", false, true));
    }

    #[test]
    fn script_carries_only_sites_with_css() {
        assert_eq!(css_script(&sites(&[("a.com", Some(true), " ")])), "");
        let script = css_script(&sites(&[("A.com", None, "body { color: \"red\" }")]));
        assert!(
            script.contains(r#"{"a.com":"body { color: \"red\" }"}"#),
            "{script}"
        );
        // Both scripts share the isolated-world state.
        assert!(script.contains("window.__ionSiteCss ??="));
        assert!(clear_css_script().contains("window.__ionSiteCss.style?.remove()"));
    }
}
