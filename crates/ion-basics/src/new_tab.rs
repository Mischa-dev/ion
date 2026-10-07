//! The new-tab page: a greeting and a grid of site shortcuts.
//!
//! The shortcuts are a fixed default set for now; the config work makes them
//! configurable and the history store can add most-visited sites later.

/// One tile on the new-tab page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    pub title: String,
    pub url: String,
}

impl Shortcut {
    pub fn new(title: &str, url: &str) -> Self {
        Self {
            title: title.to_owned(),
            url: url.to_owned(),
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
