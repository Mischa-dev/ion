//! The new-tab page: a greeting and a grid of site shortcuts.
//!
//! The tiles are the person's pinned shortcuts (`newTab.shortcuts` in the
//! config) followed by the sites they visit most, one tile per site. A fresh
//! profile with neither gets a small default set.

/// One tile on the new-tab page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    pub title: String,
    pub url: String,
    /// A page whose icon stands for the tile: the tile's own URL, or for a
    /// most-visited site the page visited most there (its front page may
    /// never have been opened, so has no known icon).
    pub icon_page: String,
}

impl Shortcut {
    pub fn new(title: &str, url: &str) -> Self {
        Self {
            title: title.to_owned(),
            url: url.to_owned(),
            icon_page: url.to_owned(),
        }
    }

    /// The letter shown on the tile until favicons land: the first
    /// alphanumeric character of the title, upper-cased.
    pub fn letter(&self) -> String {
        self.title
            .chars()
            .find(|c| c.is_alphanumeric())
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_else(|| "•".to_owned())
    }
}

/// The shortcuts a fresh profile starts with.
pub fn default_shortcuts() -> Vec<Shortcut> {
    vec![
        Shortcut::new("GitHub", "https://github.com/"),
        Shortcut::new("NixOS Search", "https://search.nixos.org/packages"),
        Shortcut::new("Wikipedia", "https://en.wikipedia.org/"),
        Shortcut::new("YouTube", "https://www.youtube.com/"),
        Shortcut::new("Hacker News", "https://news.ycombinator.com/"),
        Shortcut::new("Reddit", "https://www.reddit.com/"),
    ]
}

/// The tiles to show: `pinned` first, then one tile per site from `visited`
/// (pages, best first) whose site is not already shown, up to `max` in all.
/// Most-visited tiles open the site's front page and use its title when the
/// front page is among `visited`, otherwise the host name. With nothing
/// pinned or visited, the defaults stand in.
pub fn tiles(
    pinned: &[Shortcut],
    visited: &[Shortcut],
    most_visited: bool,
    max: usize,
) -> Vec<Shortcut> {
    let mut out: Vec<Shortcut> = pinned
        .iter()
        .filter(|s| !s.url.trim().is_empty())
        .map(|s| {
            let title = if s.title.trim().is_empty() {
                crate::display_host(&s.url).unwrap_or_else(|| s.url.clone())
            } else {
                s.title.clone()
            };
            Shortcut::new(&title, s.url.trim())
        })
        .take(max)
        .collect();
    if most_visited {
        let mut hosts: Vec<String> = out
            .iter()
            .filter_map(|s| crate::display_host(&s.url))
            .collect();
        for page in visited {
            if out.len() >= max {
                break;
            }
            let Some((host, front)) = front_page(&page.url) else {
                continue;
            };
            if hosts.contains(&host) {
                continue;
            }
            let title = visited
                .iter()
                .find(|p| p.url == front && !p.title.trim().is_empty())
                .map_or_else(|| host.clone(), |p| p.title.trim().to_owned());
            out.push(Shortcut {
                icon_page: page.url.clone(),
                ..Shortcut::new(&title, &front)
            });
            hosts.push(host);
        }
    }
    if out.is_empty() {
        out = default_shortcuts();
        out.truncate(max);
    }
    out
}

/// The display host and front-page URL of a web page; `None` for pages that
/// are not http(s) or have no host.
fn front_page(url: &str) -> Option<(String, String)> {
    let parsed = url::Url::parse(url).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    let host = crate::display_host(url)?;
    let mut front = parsed;
    front.set_path("/");
    front.set_query(None);
    front.set_fragment(None);
    Some((host, front.to_string()))
}

/// Greeting for the local hour of day (0–23).
pub fn greeting(hour: u32) -> &'static str {
    match hour {
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        18..=22 => "Good evening",
        _ => "Hello, night owl",
    }
}

/// True for URLs that should show the new-tab page instead of web content.
pub fn is_new_tab_url(url: &str) -> bool {
    let url = url.trim();
    url.is_empty() || url == "about:blank" || url == "ion://newtab"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters() {
        assert_eq!(Shortcut::new("github", "x").letter(), "G");
        assert_eq!(Shortcut::new("  ÿes", "x").letter(), "Ÿ");
        assert_eq!(Shortcut::new("(9gag)", "x").letter(), "9");
        assert_eq!(Shortcut::new("", "x").letter(), "•");
    }

    #[test]
    fn defaults_are_valid_urls() {
        let shortcuts = default_shortcuts();
        assert!(!shortcuts.is_empty());
        for s in shortcuts {
            assert!(url::Url::parse(&s.url).is_ok(), "{}", s.url);
        }
    }

    fn s(title: &str, url: &str) -> Shortcut {
        Shortcut::new(title, url)
    }

    /// Title and URL of each tile.
    fn shown(tiles: Vec<Shortcut>) -> Vec<(String, String)> {
        tiles.into_iter().map(|t| (t.title, t.url)).collect()
    }

    fn pair(title: &str, url: &str) -> (String, String) {
        (title.to_owned(), url.to_owned())
    }

    #[test]
    fn fresh_profile_gets_defaults() {
        assert_eq!(tiles(&[], &[], true, 8), default_shortcuts());
        assert_eq!(tiles(&[], &[], true, 2).len(), 2);
    }

    #[test]
    fn pinned_come_first_then_one_tile_per_site() {
        let pinned = [
            s("Mail", "https://mail.example/inbox"),
            s("", "https://x.org/"),
        ];
        let visited = [
            s("PR #7", "https://github.com/Mischa-dev/ion/pull/7"),
            s("Inbox", "https://mail.example/inbox?id=3"),
            s("GitHub", "https://github.com/"),
            s("A page", "http://localhost:8765/long.html"),
            s("", "file:///tmp/a.html"),
        ];
        let result = tiles(&pinned, &visited, true, 8);
        // The icon comes from the page actually visited on that site.
        assert_eq!(
            result[2].icon_page,
            "https://github.com/Mischa-dev/ion/pull/7"
        );
        assert_eq!(
            shown(result),
            [
                pair("Mail", "https://mail.example/inbox"),
                pair("x.org", "https://x.org/"),
                // The front page's own title wins over the deep link's.
                pair("GitHub", "https://github.com/"),
                pair("localhost", "http://localhost:8765/"),
            ]
        );
    }

    #[test]
    fn most_visited_can_be_turned_off_and_capped() {
        let visited = [s("A", "https://a.example/x"), s("B", "https://b.example/")];
        assert_eq!(
            tiles(&[s("P", "https://p.example/")], &visited, false, 8),
            [s("P", "https://p.example/")]
        );
        assert_eq!(
            shown(tiles(&[], &visited, true, 1)),
            [pair("a.example", "https://a.example/")]
        );
        // Turning it off with nothing pinned still shows something.
        assert_eq!(tiles(&[], &visited, false, 8), default_shortcuts());
    }

    #[test]
    fn greetings_cover_the_day() {
        assert_eq!(greeting(8), "Good morning");
        assert_eq!(greeting(12), "Good afternoon");
        assert_eq!(greeting(20), "Good evening");
        assert_eq!(greeting(2), "Hello, night owl");
        assert_eq!(greeting(23), "Hello, night owl");
    }

    #[test]
    fn new_tab_urls() {
        assert!(is_new_tab_url(""));
        assert!(is_new_tab_url("about:blank"));
        assert!(!is_new_tab_url("https://example.com/"));
    }
}
