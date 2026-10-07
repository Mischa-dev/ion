//! Page zoom: the zoom steps Ctrl+Plus / Ctrl+Minus walk through, and a
//! per-site memory so a site opens at the zoom it was last left at.

use std::collections::BTreeMap;

/// Zoom factors in the order Ctrl+Plus walks through them (Chromium's steps).
pub const STEPS: [f64; 17] = [
    0.25, 0.33, 0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0, 5.0,
];

/// The default zoom factor.
pub const DEFAULT: f64 = 1.0;

/// Factors closer than this count as the same step, so a factor that went
/// through `f32` or a percentage round trip still lands on its step.
const EPSILON: f64 = 0.005;

/// The next step above `factor`, or the largest step if already there.
pub fn step_in(factor: f64) -> f64 {
    STEPS
        .iter()
        .copied()
        .find(|step| *step > factor + EPSILON)
        .unwrap_or(STEPS[STEPS.len() - 1])
}

/// The next step below `factor`, or the smallest step if already there.
pub fn step_out(factor: f64) -> f64 {
    STEPS
        .iter()
        .rev()
        .copied()
        .find(|step| *step < factor - EPSILON)
        .unwrap_or(STEPS[0])
}

/// True if `factor` is the default zoom.
pub fn is_default(factor: f64) -> bool {
    (factor - DEFAULT).abs() < EPSILON
}

/// The factor as a whole percentage, for display ("110%").
pub fn percent(factor: f64) -> i32 {
    (factor * 100.0).round() as i32
}

/// Remembers a zoom factor per site (keyed by host, `www.` ignored).
///
/// Sites at the default zoom are not stored. The memory serializes to a small
/// line-based text format, one `host factor` pair per line.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SiteZoom {
    sites: BTreeMap<String, f64>,
}

impl SiteZoom {
    pub fn new() -> Self {
        Self::default()
    }

    /// The remembered factor for `url`'s site, or the default.
    pub fn factor_for(&self, url: &str) -> f64 {
        crate::display_host(url)
            .and_then(|host| self.sites.get(&host).copied())
            .unwrap_or(DEFAULT)
    }

    /// Remember `factor` for `url`'s site. Returns true if the memory changed.
    /// URLs without a host (new-tab page, local files) are not remembered.
    pub fn remember(&mut self, url: &str, factor: f64) -> bool {
        let Some(host) = crate::display_host(url) else {
            return false;
        };
        let factor = factor.clamp(STEPS[0], STEPS[STEPS.len() - 1]);
        if is_default(factor) {
            self.sites.remove(&host).is_some()
        } else {
            let previous = self.sites.insert(host, factor);
            previous.is_none_or(|old| (old - factor).abs() >= EPSILON)
        }
    }

    /// Number of sites with a non-default zoom.
    pub fn len(&self) -> usize {
        self.sites.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sites.is_empty()
    }

    /// Parse the text format written by [`SiteZoom::to_text`]. Malformed lines
    /// are skipped, so a hand-edited file never loses the rest.
    pub fn from_text(text: &str) -> Self {
        let mut zoom = Self::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let (Some(host), Some(factor), None) = (parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let Ok(factor) = factor.parse::<f64>() else {
                continue;
            };
            if factor.is_finite() {
                zoom.remember(&format!("https://{host}/"), factor);
            }
        }
        zoom
    }

    /// Serialize to the line-based text format, sorted by host.
    pub fn to_text(&self) -> String {
        let mut out = String::from("# Ion per-site zoom: host factor\n");
        for (host, factor) in &self.sites {
            out.push_str(&format!("{host} {factor}\n"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_walk_up_and_down() {
        assert_eq!(step_in(1.0), 1.1);
        assert_eq!(step_out(1.0), 0.9);
        assert_eq!(step_in(0.25), 0.33);
        assert_eq!(step_out(1.1), 1.0);
    }

    #[test]
    fn steps_stop_at_the_ends() {
        assert_eq!(step_in(5.0), 5.0);
        assert_eq!(step_out(0.25), 0.25);
        assert_eq!(step_in(9.0), 5.0);
        assert_eq!(step_out(0.1), 0.25);
    }

    #[test]
    fn off_step_factors_snap_to_the_next_step() {
        assert_eq!(step_in(1.03), 1.1);
        assert_eq!(step_out(1.03), 1.0);
        // An f32 round trip of 1.1 is still treated as 1.1.
        assert_eq!(step_in(f64::from(1.1_f32)), 1.25);
    }

    #[test]
    fn percent_rounds() {
        assert_eq!(percent(1.0), 100);
        assert_eq!(percent(0.33), 33);
        assert_eq!(percent(1.25), 125);
    }

    #[test]
    fn site_zoom_is_per_host() {
        let mut zoom = SiteZoom::new();
        assert!(zoom.remember("https://www.example.com/a", 1.25));
        assert_eq!(zoom.factor_for("https://example.com/b"), 1.25);
        assert_eq!(zoom.factor_for("https://other.org/"), DEFAULT);
    }

    #[test]
    fn default_zoom_forgets_the_site() {
        let mut zoom = SiteZoom::new();
        zoom.remember("https://example.com/", 1.5);
        assert!(zoom.remember("https://example.com/", 1.0));
        assert!(zoom.is_empty());
        assert!(!zoom.remember("https://example.com/", 1.0));
    }

    #[test]
    fn remember_reports_unchanged() {
        let mut zoom = SiteZoom::new();
        assert!(zoom.remember("https://example.com/", 1.5));
        assert!(!zoom.remember("https://example.com/x", 1.5));
    }

    #[test]
    fn hostless_urls_are_not_remembered() {
        let mut zoom = SiteZoom::new();
        assert!(!zoom.remember("about:blank", 2.0));
        assert!(!zoom.remember("file:///tmp/a.html", 2.0));
        assert!(zoom.is_empty());
    }

    #[test]
    fn text_round_trip() {
        let mut zoom = SiteZoom::new();
        zoom.remember("https://example.com/", 1.25);
        zoom.remember("https://docs.rs/", 0.9);
        let text = zoom.to_text();
        assert_eq!(SiteZoom::from_text(&text), zoom);
    }

    #[test]
    fn malformed_lines_are_skipped() {
        let zoom = SiteZoom::from_text(
            "# comment\nexample.com 1.5\nbroken\nnan.org NaN\nx.org abc\ny.org 2 extra\n",
        );
        assert_eq!(zoom.len(), 1);
        assert_eq!(zoom.factor_for("https://example.com/"), 1.5);
    }
}
