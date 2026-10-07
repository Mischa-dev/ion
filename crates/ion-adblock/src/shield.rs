//! The per-request decision plus the state the UI shows: blocked counters and
//! the per-site switch.

use std::collections::HashMap;

use crate::blocker::{PageCosmetics, hiding_css};
use crate::sites::site_of;
use crate::{Blocker, RequestInfo, ResourceType, SiteSettings, Verdict};

/// Owns the compiled [`Blocker`] and decides about each request.
///
/// Counters are kept per site and start over whenever a page on that site is
/// loaded, so the number next to the address bar means "blocked on this page".
#[derive(Default)]
pub struct Shield {
    blocker: Option<Blocker>,
    enabled: bool,
    sites: SiteSettings,
    blocked: HashMap<String, u32>,
    total: u64,
}

impl Shield {
    pub fn new(sites: SiteSettings) -> Self {
        Self {
            enabled: true,
            sites,
            ..Self::default()
        }
    }

    /// Swap in a newly compiled blocker (after a list update).
    pub fn set_blocker(&mut self, blocker: Blocker) {
        self.blocker = Some(blocker);
    }

    /// Drop the blocker, e.g. when every filter list was removed.
    pub fn clear_blocker(&mut self) {
        self.blocker = None;
    }

    /// True once filter lists are loaded; until then nothing is blocked.
    pub fn is_ready(&self) -> bool {
        self.blocker.is_some()
    }

    /// The global switch.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn sites(&self) -> &SiteSettings {
        &self.sites
    }

    /// Whether blocking applies on the page at `page_url`.
    pub fn is_enabled_on(&self, page_url: &str) -> bool {
        site_of(page_url).is_none_or(|site| self.sites.is_enabled(&site))
    }

    /// Switch blocking on or off for the site of `page_url`. Returns false
    /// (and changes nothing) for pages without a site, like `about:blank`,
    /// and when switching off a public suffix like `github.io`.
    pub fn set_enabled_on(&mut self, page_url: &str, enabled: bool) -> bool {
        let Some(site) = site_of(page_url) else {
            return false;
        };
        self.sites.set_enabled(&site, enabled)
    }

    /// Requests blocked on the current page of `page_url`'s site.
    pub fn blocked_on(&self, page_url: &str) -> u32 {
        site_of(page_url)
            .and_then(|site| self.blocked.get(&site).copied())
            .unwrap_or(0)
    }

    /// Requests blocked since Ion started.
    pub fn total_blocked(&self) -> u64 {
        self.total
    }

    /// A style sheet hiding the ad elements that the lists name for the page
    /// at `page_url`. Empty when blocking is off there.
    pub fn page_css(&self, page_url: &str) -> String {
        self.cosmetics(page_url)
            .map(|(_, page)| hiding_css(&page.hide))
            .unwrap_or_default()
    }

    /// A style sheet hiding elements with these classes and ids that generic
    /// rules (`##.ad-banner`, for every site) name. The page collects them
    /// from its DOM. Empty when blocking or generic rules are off there.
    pub fn generic_css<'a>(
        &self,
        page_url: &str,
        classes: impl IntoIterator<Item = &'a str>,
        ids: impl IntoIterator<Item = &'a str>,
    ) -> String {
        self.cosmetics(page_url)
            .map(|(blocker, page)| hiding_css(&blocker.generic_selectors(&page, classes, ids)))
            .unwrap_or_default()
    }

    fn cosmetics(&self, page_url: &str) -> Option<(&Blocker, PageCosmetics)> {
        let blocker = self.blocker.as_ref()?;
        if !self.enabled || !is_web_url(page_url) || !self.is_enabled_on(page_url) {
            return None;
        }
        Some((blocker, blocker.page_cosmetics(page_url)))
    }

    /// Decide what to do with `request`, updating the counters. Pages
    /// themselves (main-frame navigations) are never blocked, only stripped
    /// of tracking parameters; loading one resets its site's counter.
    pub fn decide(&mut self, request: &RequestInfo<'_>) -> Verdict {
        let is_page = request.resource == ResourceType::MainFrame;
        if is_page {
            if let Some(site) = site_of(request.url) {
                self.blocked.remove(&site);
            }
        }
        if !self.enabled || !is_web_url(request.url) {
            return Verdict::Allow;
        }
        let Some(blocker) = &self.blocker else {
            return Verdict::Allow;
        };
        let site = site_of(request.first_party);
        if site.as_ref().is_some_and(|s| !self.sites.is_enabled(s)) {
            return Verdict::Allow;
        }
        match blocker.check(request) {
            // People typed or clicked the page; load it even if it is an ad.
            Verdict::Block if is_page => Verdict::Allow,
            Verdict::Block => {
                self.total += 1;
                if let Some(site) = site {
                    *self.blocked.entry(site).or_default() += 1;
                }
                Verdict::Block
            }
            // QtWebEngine can only redirect requests without a body, so a
            // POST keeps its tracking parameters rather than break.
            Verdict::Rewrite(_) if !is_bodyless(request.method) => Verdict::Allow,
            verdict => verdict,
        }
    }
}

fn is_bodyless(method: &str) -> bool {
    method.eq_ignore_ascii_case("GET") || method.eq_ignore_ascii_case("HEAD")
}

/// Only http(s) and websocket traffic goes through the filters; `data:`,
/// `blob:`, `qrc:`, `chrome:` and friends never do.
fn is_web_url(url: &str) -> bool {
    let scheme = url.split(':').next().unwrap_or("").to_ascii_lowercase();
    matches!(scheme.as_str(), "http" | "https" | "ws" | "wss")
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "https://www.news.test/article";

    fn shield() -> Shield {
        let mut shield = Shield::new(SiteSettings::default());
        shield.set_blocker(Blocker::from_lists(["||ads.test^\n"]));
        shield
    }

    fn ad(first_party: &str) -> RequestInfo<'_> {
        RequestInfo::new("https://ads.test/a.js", first_party, ResourceType::Script)
    }

    #[test]
    fn nothing_is_blocked_before_lists_load() {
        let mut shield = Shield::new(SiteSettings::default());
        assert!(!shield.is_ready());
        assert!(!shield.decide(&ad(PAGE)).is_block());
        assert_eq!(shield.total_blocked(), 0);
    }

    #[test]
    fn blocks_and_counts_per_site() {
        let mut shield = shield();
        assert!(shield.decide(&ad(PAGE)).is_block());
        assert!(shield.decide(&ad("https://news.test/other")).is_block());
        assert!(shield.decide(&ad("https://blog.test/")).is_block());
        assert!(
            !shield
                .decide(&RequestInfo::new(
                    "https://news.test/app.js",
                    PAGE,
                    ResourceType::Script
                ))
                .is_block()
        );
        assert_eq!(shield.blocked_on(PAGE), 2);
        assert_eq!(shield.blocked_on("https://blog.test/x"), 1);
        assert_eq!(shield.blocked_on("about:blank"), 0);
        assert_eq!(shield.total_blocked(), 3);
    }

    #[test]
    fn loading_a_page_resets_its_counter_and_is_never_blocked() {
        let mut shield = shield();
        shield.decide(&ad(PAGE));
        let nav = RequestInfo::new(
            "https://news.test/next",
            "https://news.test/next",
            ResourceType::MainFrame,
        );
        assert!(!shield.decide(&nav).is_block());
        assert_eq!(shield.blocked_on(PAGE), 0);
        assert_eq!(shield.total_blocked(), 1);

        // Even a page on a blocked domain loads; people typed or clicked it.
        let blocked_page = RequestInfo::new(
            "https://ads.test/",
            "https://ads.test/",
            ResourceType::MainFrame,
        );
        assert!(!shield.decide(&blocked_page).is_block());
    }

    #[test]
    fn per_site_switch() {
        let mut shield = shield();
        assert!(shield.set_enabled_on(PAGE, false));
        assert!(!shield.is_enabled_on("https://news.test/"));
        assert!(!shield.decide(&ad(PAGE)).is_block());
        assert!(shield.decide(&ad("https://blog.test/")).is_block());

        assert!(shield.set_enabled_on("https://news.test/", true));
        assert!(shield.decide(&ad(PAGE)).is_block());

        assert!(!shield.set_enabled_on("about:blank", false));
        assert!(shield.is_enabled_on("about:blank"));
    }

    #[test]
    fn global_switch() {
        let mut shield = shield();
        shield.set_enabled(false);
        assert!(!shield.decide(&ad(PAGE)).is_block());
        shield.set_enabled(true);
        assert!(shield.decide(&ad(PAGE)).is_block());
    }

    #[test]
    fn rewrites_are_not_counted_as_blocked() {
        let mut shield = Shield::new(SiteSettings::default());
        shield.set_blocker(Blocker::from_lists(["$removeparam=fbclid\n"]));
        let xhr = RequestInfo::new("https://cdn.test/api?fbclid=x", PAGE, ResourceType::Xhr);
        assert_eq!(
            shield.decide(&xhr),
            Verdict::Rewrite("https://cdn.test/api".to_owned())
        );
        // Pages lose tracking parameters too.
        let page = "https://news.test/story?id=7&fbclid=x";
        let nav = RequestInfo::new(page, page, ResourceType::MainFrame);
        assert_eq!(
            shield.decide(&nav),
            Verdict::Rewrite("https://news.test/story?id=7".to_owned())
        );
        assert_eq!(shield.total_blocked(), 0);

        let post = RequestInfo {
            method: "POST",
            ..xhr
        };
        assert_eq!(shield.decide(&post), Verdict::Allow);
    }

    #[test]
    fn only_web_requests_are_filtered() {
        let mut shield = Shield::new(SiteSettings::default());
        shield.set_blocker(Blocker::from_lists(["ads\n"]));
        let data = RequestInfo::new("data:text/plain,ads", PAGE, ResourceType::Image);
        assert!(!shield.decide(&data).is_block());
        let ws = RequestInfo::new("wss://ads.test/socket", PAGE, ResourceType::WebSocket);
        assert!(shield.decide(&ws).is_block());
    }

    #[test]
    fn cosmetic_css_follows_the_switches() {
        let list = "news.test##.sponsored\n##.ad-banner\n";
        let mut shield = Shield::new(SiteSettings::default());
        assert_eq!(shield.page_css(PAGE), "");
        shield.set_blocker(Blocker::from_lists([list]));
        assert_eq!(
            shield.page_css(PAGE),
            ".sponsored{display:none!important}\n"
        );
        assert_eq!(
            shield.generic_css(PAGE, ["ad-banner", "story"], []),
            ".ad-banner{display:none!important}\n"
        );
        assert_eq!(shield.page_css("file:///home/x.html"), "");

        assert!(shield.set_enabled_on(PAGE, false));
        assert_eq!(shield.page_css(PAGE), "");
        assert_eq!(shield.generic_css(PAGE, ["ad-banner"], []), "");
        assert!(shield.set_enabled_on(PAGE, true));

        shield.set_enabled(false);
        assert_eq!(shield.page_css(PAGE), "");
    }
}
