//! Ion's own pages, at `ion://<page>`: Settings, History, Downloads,
//! Bookmarks and the welcome page.
//!
//! The `ion` scheme serves each one an empty document with its title, so it
//! is a real history entry (Back, Reload, tab titles and session restore all
//! work), and the QML page for it is drawn over the tab.

/// One built-in page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    /// The URL's host: `ion://settings`.
    pub id: &'static str,
    pub title: &'static str,
}

pub const SCHEME: &str = "ion";

pub const PAGES: &[Page] = &[
    Page {
        id: "settings",
        title: "Settings",
    },
    Page {
        id: "history",
        title: "History",
    },
    Page {
        id: "downloads",
        title: "Downloads",
    },
    Page {
        id: "bookmarks",
        title: "Bookmarks",
    },
    Page {
        id: "welcome",
        title: "Welcome to Ion",
    },
    // The new-tab page draws itself over this one (NewTabPage.qml); the tab
    // strip shows it as "New Tab".
    Page {
        id: "newtab",
        title: "",
    },
];

/// The page `url` shows and the rest of its path (`ion://settings/privacy`
/// is `settings` and `privacy`), or `None` for any other URL.
pub fn parse(url: &str) -> Option<(Page, &str)> {
    let rest = url.strip_prefix("ion://")?;
    let rest = rest.split(['?', '#']).next().unwrap_or_default();
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let page = PAGES
        .iter()
        .find(|p| p.id.eq_ignore_ascii_case(host))
        .copied()?;
    Some((page, path.trim_matches('/')))
}

/// `ion://<id>`, plus `/<path>` when there is one.
pub fn url(id: &str, path: &str) -> String {
    if path.is_empty() {
        format!("{SCHEME}://{id}")
    } else {
        format!("{SCHEME}://{id}/{path}")
    }
}

/// The document the scheme serves for `url`: empty, titled, and in the
/// background color given so nothing flashes before the page is drawn over it.
pub fn document(url: &str, background: &str) -> Option<String> {
    let (page, _) = parse(url)?;
    let escape = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let background: String = background
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '#')
        .collect();
    Some(format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title>\
         <style>html{{background:{background};color-scheme:light dark}}</style></head><body></body></html>",
        escape(page.title)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_page_and_path() {
        let (page, path) = parse("ion://settings/privacy").unwrap();
        assert_eq!((page.id, path), ("settings", "privacy"));
        let (page, path) = parse("ion://History").unwrap();
        assert_eq!((page.id, path), ("history", ""));
        let (page, path) = parse("ion://settings/?q=1#x").unwrap();
        assert_eq!((page.id, path), ("settings", ""));
        assert!(parse("ion://nope").is_none());
        assert!(parse("https://settings").is_none());
    }

    #[test]
    fn builds_urls() {
        assert_eq!(url("settings", ""), "ion://settings");
        assert_eq!(url("settings", "search"), "ion://settings/search");
    }

    #[test]
    fn document_has_the_title_and_a_safe_color() {
        let html = document("ion://downloads", "#112233;}</style><script>").unwrap();
        assert!(html.contains("<title>Downloads</title>"));
        assert!(html.contains("background:#112233stylescript"));
        assert!(!html.contains("<script>"));
        assert!(document("ion://unknown", "#000").is_none());
    }
}
