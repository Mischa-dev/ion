//! Web panels: sites kept open in the side panel, like Vivaldi's.

use serde::{Deserialize, Serialize};

/// Narrowest and widest the side panel can be dragged, in logical pixels.
pub const MIN_WIDTH: u32 = 280;
pub const MAX_WIDTH: u32 = 900;
const DEFAULT_WIDTH: u32 = 380;

/// One site in the side panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebPanel {
    /// Stable id, used to keep each panel's page alive between openings.
    pub id: String,
    pub url: String,
    /// Shown as the panel's tooltip; the site's host if empty.
    #[serde(default)]
    pub title: String,
}

impl WebPanel {
    /// The title, or the URL's host when there is none.
    pub fn label(&self) -> String {
        if !self.title.trim().is_empty() {
            return self.title.trim().to_owned();
        }
        url::Url::parse(&self.url)
            .ok()
            .and_then(|u| {
                u.host_str()
                    .map(|h| h.trim_start_matches("www.").to_owned())
            })
            .unwrap_or_else(|| self.url.clone())
    }
}

/// The side panel's state: web panels in order, its width, and what was
/// open last ("notes", a web panel's id, or "" when closed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Panels {
    pub panels: Vec<WebPanel>,
    pub width: u32,
    pub open: String,
    next_id: u64,
}

impl Default for Panels {
    fn default() -> Self {
        Self {
            panels: Vec::new(),
            width: DEFAULT_WIDTH,
            open: String::new(),
            next_id: 1,
        }
    }
}

/// Only web pages can be panels.
fn web_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url.trim()).ok()?;
    matches!(parsed.scheme(), "http" | "https").then(|| parsed.to_string())
}

impl Panels {
    /// Read from JSON; anything unreadable starts afresh.
    pub fn from_json(text: &str) -> Self {
        let mut panels: Self = serde_json::from_str(text).unwrap_or_default();
        panels.width = panels.width.clamp(MIN_WIDTH, MAX_WIDTH);
        panels
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Add `url` as a panel, or return the existing panel's id if it is
    /// already one. `None` for addresses that aren't web pages.
    pub fn add(&mut self, url: &str, title: &str) -> Option<String> {
        let url = web_url(url)?;
        if let Some(panel) = self.panels.iter().find(|p| p.url == url) {
            return Some(panel.id.clone());
        }
        let id = format!("web-{}", self.next_id.max(1));
        self.next_id = self.next_id.max(1) + 1;
        self.panels.push(WebPanel {
            id: id.clone(),
            url,
            title: title.trim().to_owned(),
        });
        Some(id)
    }

    /// Remove a panel; closes the side panel if it was showing.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.panels.len();
        self.panels.retain(|p| p.id != id);
        if self.open == id {
            self.open.clear();
        }
        self.panels.len() != before
    }

    /// Move a panel to `index` in the strip.
    pub fn move_to(&mut self, id: &str, index: usize) -> bool {
        let Some(from) = self.panels.iter().position(|p| p.id == id) else {
            return false;
        };
        let panel = self.panels.remove(from);
        let index = index.min(self.panels.len());
        self.panels.insert(index, panel);
        from != index
    }

    pub fn set_width(&mut self, width: u32) {
        self.width = width.clamp(MIN_WIDTH, MAX_WIDTH);
    }

    /// Show "notes" or a web panel, or close ("" or an unknown id).
    pub fn show(&mut self, what: &str) {
        self.open = if what == "notes" || self.panels.iter().any(|p| p.id == what) {
            what.to_owned()
        } else {
            String::new()
        };
    }

    /// Open `what` unless it is what's showing, which closes it.
    pub fn toggle(&mut self, what: &str) {
        if self.open == what {
            self.open.clear();
        } else {
            self.show(what);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_web_pages_once() {
        let mut panels = Panels::default();
        let id = panels.add("https://music.example/", "Music").unwrap();
        assert_eq!(panels.add("https://music.example/", ""), Some(id.clone()));
        assert_eq!(panels.add("file:///etc/passwd", ""), None);
        assert_eq!(panels.add("not a url", ""), None);
        let chat = panels.add("https://chat.example", "").unwrap();
        assert_ne!(id, chat);
        assert_eq!(panels.panels[1].label(), "chat.example");
        assert_eq!(panels.panels[0].label(), "Music");
    }

    #[test]
    fn ids_are_not_reused_after_removal() {
        let mut panels = Panels::default();
        let a = panels.add("https://a.example/", "").unwrap();
        panels.show(&a);
        assert!(panels.remove(&a));
        assert_eq!(panels.open, "");
        let b = panels.add("https://b.example/", "").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn show_and_toggle() {
        let mut panels = Panels::default();
        panels.toggle("notes");
        assert_eq!(panels.open, "notes");
        panels.toggle("notes");
        assert_eq!(panels.open, "");
        panels.show("web-9");
        assert_eq!(panels.open, "");
    }

    #[test]
    fn reorders_and_clamps_width() {
        let mut panels = Panels::default();
        let a = panels.add("https://a.example/", "").unwrap();
        panels.add("https://b.example/", "").unwrap();
        assert!(panels.move_to(&a, 5));
        assert_eq!(panels.panels[1].id, a);
        panels.set_width(10);
        assert_eq!(panels.width, MIN_WIDTH);
    }

    #[test]
    fn round_trips_and_survives_garbage() {
        let mut panels = Panels::default();
        panels.add("https://a.example/", "A").unwrap();
        panels.set_width(500);
        let again = Panels::from_json(&panels.to_json());
        assert_eq!(again, panels);
        assert_eq!(Panels::from_json("{oops"), Panels::default());
        let next = Panels::from_json(&panels.to_json())
            .add("https://b.example/", "")
            .unwrap();
        assert_eq!(next, "web-2");
    }
}
