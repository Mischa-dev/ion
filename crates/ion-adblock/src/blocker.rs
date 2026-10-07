//! The compiled filter engine.

use std::collections::HashSet;
use std::fmt;

use adblock::Engine;
use adblock::lists::{FilterFormat, FilterSet, ParseOptions, RuleTypes};
use adblock::request::Request;

use crate::RequestInfo;

/// Filter lists compiled into an adblock-rust engine, ready to match requests.
pub struct Blocker {
    engine: Engine,
}

/// What to do with a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Block,
    /// Load this URL instead: the original with tracking parameters removed
    /// by a `$removeparam` rule.
    Rewrite(String),
}

impl Verdict {
    pub fn is_block(&self) -> bool {
        *self == Self::Block
    }
}

/// The cached engine could not be loaded, usually because it was written by a
/// different adblock-rust version. The caller rebuilds it from the lists.
#[derive(Debug)]
pub struct DeserializeError(String);

impl fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cached adblock engine is unusable: {}", self.0)
    }
}

impl std::error::Error for DeserializeError {}

impl Blocker {
    /// Compile the given filter list texts, each either in Adblock Plus syntax
    /// or a hosts file. Unparseable lines are skipped.
    pub fn from_lists<'a>(lists: impl IntoIterator<Item = &'a str>) -> Self {
        let mut set = FilterSet::new(false);
        for text in lists {
            let options = ParseOptions {
                format: format_of(text),
                rule_types: RuleTypes::All,
                ..ParseOptions::default()
            };
            set.add_filter_list(crate::directives::preprocess(text), options);
        }
        Self {
            engine: Engine::new_with_filter_set(set),
        }
    }

    /// A compact binary form of the engine that loads much faster than parsing
    /// the lists again.
    pub fn serialize(&self) -> Vec<u8> {
        self.engine.serialize()
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, DeserializeError> {
        let mut engine = Engine::default();
        engine
            .deserialize(bytes)
            .map_err(|e| DeserializeError(format!("{e:?}")))?;
        Ok(Self { engine })
    }

    /// Whether the filters say this request should be blocked.
    pub fn should_block(&self, request: &RequestInfo<'_>) -> bool {
        self.check(request).is_block()
    }

    /// What the filters say to do with this request.
    pub fn check(&self, request: &RequestInfo<'_>) -> Verdict {
        let Ok(req) = Request::new(
            request.url,
            request.first_party,
            request.resource.filter_type(),
            request.method,
        ) else {
            return Verdict::Allow;
        };
        let result = self.engine.check_network_request(&req);
        if result.should_block() {
            Verdict::Block
        } else {
            match result.rewritten_url {
                Some(url) if url != request.url => Verdict::Rewrite(url),
                _ => Verdict::Allow,
            }
        }
    }
}

/// The element-hiding (`##`) rules that apply to one page.
#[derive(Debug, Default)]
pub struct PageCosmetics {
    /// Selectors to hide on this page, sorted.
    pub hide: Vec<String>,
    /// Whether generic rules (`##.ad`, which apply on every site) are used
    /// here; false on pages with a `$generichide` exception.
    pub generic: bool,
    /// Class and id selectors this page excepts from the generic rules.
    exceptions: HashSet<String>,
}

impl Blocker {
    /// The element-hiding rules specific to the page at `url`.
    pub fn page_cosmetics(&self, url: &str) -> PageCosmetics {
        let resources = self.engine.url_cosmetic_resources(url);
        let mut hide: Vec<String> = resources
            .hide_selectors
            .into_iter()
            .filter(|s| is_safe_selector(s))
            .collect();
        hide.sort();
        PageCosmetics {
            hide,
            generic: !resources.generichide,
            exceptions: resources.exceptions,
        }
    }

    /// Generic rules matching any of these classes and ids, which the page
    /// collects from its DOM. Empty when the page doesn't use generic rules.
    pub fn generic_selectors<'a>(
        &self,
        page: &PageCosmetics,
        classes: impl IntoIterator<Item = &'a str>,
        ids: impl IntoIterator<Item = &'a str>,
    ) -> Vec<String> {
        if !page.generic {
            return Vec::new();
        }
        let mut selectors: Vec<String> = self
            .engine
            .hidden_class_id_selectors(classes, ids, &page.exceptions)
            .into_iter()
            .filter(|s| is_safe_selector(s))
            .collect();
        selectors.sort();
        selectors
    }
}

/// A style sheet hiding everything `selectors` match. One rule per selector,
/// so a selector the browser doesn't understand only voids itself.
pub fn hiding_css(selectors: &[String]) -> String {
    selectors
        .iter()
        .map(|s| format!("{s}{{display:none!important}}\n"))
        .collect()
}

/// Selectors go into a style sheet verbatim, so one that could close its rule
/// and start another, open a comment or swallow the rule's own block is
/// dropped. Braces, brackets and `/*` are fine inside quoted strings, and CSS
/// escapes (`.sm\:block`) anywhere; a dangling escape, an unterminated string
/// or an unclosed `(` or `[` is not.
fn is_safe_selector(selector: &str) -> bool {
    let mut quote = None;
    // `(` and `[` opened outside strings and not yet closed.
    let mut open = Vec::new();
    let mut chars = selector.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quote) {
            ('(' | '[', None) => open.push(c),
            (')' | ']', None) => {
                let opener = if c == ')' { '(' } else { '[' };
                if open.pop() != Some(opener) {
                    return false;
                }
            }
            ('\\', _) => {
                if chars.next().is_none() {
                    return false;
                }
            }
            ('"' | '\'', None) => quote = Some(c),
            (c, Some(q)) if c == q => quote = None,
            ('\n' | '\r' | '\x0c', Some(_)) => return false,
            ('{' | '}', None) => return false,
            ('/', None) if chars.peek() == Some(&'*') => return false,
            _ => {}
        }
    }
    !selector.is_empty() && quote.is_none() && open.is_empty()
}

/// Hosts files (`0.0.0.0 ads.example`) need their own parser; the first rule
/// line tells them apart from Adblock Plus syntax.
fn format_of(text: &str) -> FilterFormat {
    let first_rule = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with(['!', '#', '[']));
    let is_hosts = first_rule
        .and_then(|line| line.split_whitespace().next())
        .is_some_and(|addr| addr.parse::<std::net::IpAddr>().is_ok());
    if is_hosts {
        FilterFormat::Hosts
    } else {
        FilterFormat::Standard
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ResourceType;

    const LIST: &str = "\
! Title: test list
||ads.example^
/banner/*$image
||tracker.test^$third-party
@@||ads.example/allowed.js
";

    fn script(url: &str) -> RequestInfo<'_> {
        RequestInfo::new(url, "https://news.example.org/", ResourceType::Script)
    }

    #[test]
    fn blocks_matching_requests() {
        let blocker = Blocker::from_lists([LIST]);
        assert!(blocker.should_block(&script("https://ads.example/x.js")));
        assert!(blocker.should_block(&script("https://cdn.ads.example/x.js")));
        assert!(!blocker.should_block(&script("https://news.example.org/app.js")));
    }

    #[test]
    fn exceptions_win() {
        let blocker = Blocker::from_lists([LIST]);
        assert!(!blocker.should_block(&script("https://ads.example/allowed.js")));
    }

    #[test]
    fn honours_resource_type_and_party_options() {
        let blocker = Blocker::from_lists([LIST]);
        let image = RequestInfo::new(
            "https://news.example.org/banner/top.png",
            "https://news.example.org/",
            ResourceType::Image,
        );
        assert!(blocker.should_block(&image));
        let css = RequestInfo {
            resource: ResourceType::Stylesheet,
            ..image
        };
        assert!(!blocker.should_block(&css));

        let third = RequestInfo::new(
            "https://tracker.test/p",
            "https://shop.test.example/",
            ResourceType::Xhr,
        );
        assert!(blocker.should_block(&third));
        let first = RequestInfo::new(
            "https://tracker.test/p",
            "https://tracker.test/",
            ResourceType::Xhr,
        );
        assert!(!blocker.should_block(&first));
    }

    #[test]
    fn parses_hosts_files() {
        let hosts = "# hosts\n127.0.0.1 localhost\n0.0.0.0 ads.example\n";
        assert!(matches!(format_of(hosts), FilterFormat::Hosts));
        assert!(matches!(format_of(LIST), FilterFormat::Standard));
        let blocker = Blocker::from_lists([hosts]);
        assert!(blocker.should_block(&script("https://ads.example/x.js")));
        assert!(!blocker.should_block(&script("https://news.example.org/app.js")));
    }

    #[test]
    fn removeparam_rewrites_instead_of_blocking() {
        let blocker = Blocker::from_lists(["||shop.example^$removeparam=utm_source\n"]);
        let request = RequestInfo::new(
            "https://shop.example/api?id=1&utm_source=mail",
            "https://shop.example/",
            ResourceType::Xhr,
        );
        assert_eq!(
            blocker.check(&request),
            Verdict::Rewrite("https://shop.example/api?id=1".to_owned())
        );
        let clean = RequestInfo::new(
            "https://shop.example/api?id=1",
            "https://shop.example/",
            ResourceType::Xhr,
        );
        assert_eq!(blocker.check(&clean), Verdict::Allow);
    }

    #[test]
    fn firefox_only_exceptions_do_not_apply() {
        let list = "||ads.example^\n!#if env_firefox\n@@||ads.example^\n!#endif\n";
        let blocker = Blocker::from_lists([list]);
        assert!(blocker.should_block(&script("https://ads.example/x.js")));
    }

    #[test]
    fn garbage_urls_are_allowed() {
        let blocker = Blocker::from_lists([LIST]);
        assert!(!blocker.should_block(&script("not a url")));
    }

    #[test]
    fn survives_serialization() {
        let bytes = Blocker::from_lists([LIST]).serialize();
        let blocker = Blocker::deserialize(&bytes).unwrap();
        assert!(blocker.should_block(&script("https://ads.example/x.js")));
        assert!(!blocker.should_block(&script("https://ads.example/allowed.js")));
    }

    #[test]
    fn rejects_corrupt_cache() {
        assert!(Blocker::deserialize(b"definitely not an engine").is_err());
    }

    #[test]
    fn is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Blocker>();
    }

    const COSMETIC: &str = "\
news.example.org##.sponsored
example.org##div[data-ad]
##.ad-banner
###top-ad
quiet.example.org#@#.ad-banner
shop.example.org##a{color:red}body
@@||nogeneric.example.org^$generichide
";

    #[test]
    fn page_cosmetics_lists_site_specific_selectors() {
        let blocker = Blocker::from_lists([COSMETIC]);
        let page = blocker.page_cosmetics("https://news.example.org/story");
        assert_eq!(page.hide, [".sponsored", "div[data-ad]"]);
        assert!(page.generic);
        assert!(
            blocker
                .page_cosmetics("https://other.test/")
                .hide
                .is_empty()
        );
    }

    #[test]
    fn generic_selectors_match_classes_and_ids_on_the_page() {
        let blocker = Blocker::from_lists([COSMETIC]);
        let page = blocker.page_cosmetics("https://news.example.org/");
        let found = blocker.generic_selectors(&page, ["ad-banner", "content"], ["top-ad"]);
        assert_eq!(found, ["#top-ad", ".ad-banner"]);
    }

    #[test]
    fn generic_exceptions_and_generichide_are_honoured() {
        let blocker = Blocker::from_lists([COSMETIC]);
        let quiet = blocker.page_cosmetics("https://quiet.example.org/");
        assert!(
            blocker
                .generic_selectors(&quiet, ["ad-banner"], [])
                .is_empty()
        );
        let nogeneric = blocker.page_cosmetics("https://nogeneric.example.org/");
        assert!(!nogeneric.generic);
        assert!(
            blocker
                .generic_selectors(&nogeneric, ["ad-banner"], ["top-ad"])
                .is_empty()
        );
    }

    #[test]
    fn escaped_selectors_are_kept() {
        let blocker = Blocker::from_lists(["##.sm\\:block\nnews.example.org##.lg\\:ad\n"]);
        let page = blocker.page_cosmetics("https://news.example.org/");
        assert_eq!(page.hide, [r".lg\:ad"]);
        let found = blocker.generic_selectors(&page, ["sm:block"], []);
        assert_eq!(found, [r".sm\:block"]);
    }

    #[test]
    fn unsafe_selectors_never_reach_the_style_sheet() {
        let blocker = Blocker::from_lists([COSMETIC]);
        let page = blocker.page_cosmetics("https://shop.example.org/");
        assert!(page.hide.iter().all(|s| !s.contains('{')));
        assert!(!is_safe_selector("a{color:red}body"));
        assert!(!is_safe_selector("a/*"));
        assert!(is_safe_selector("div[data-ad=\"1\"] > .x"));
        assert!(is_safe_selector(r".sm\:block"));
        assert!(is_safe_selector(r".a\\"));
        assert!(!is_safe_selector(r".a\"));
        assert!(!is_safe_selector(r".a\\\"));
        assert!(is_safe_selector(r#"[data-ad="<sponsor>"]"#));
        assert!(is_safe_selector(r#"[title='a{b}/*c']"#));
        assert!(is_safe_selector(r#"[title="it's"]"#));
        assert!(!is_safe_selector(r#"[title="a{"#));
        assert!(!is_safe_selector(r#"[title="a\"]"#));
        assert!(is_safe_selector(r#"div:not([title=")"]):has(> .ad)"#));
        assert!(!is_safe_selector("div:not(.ad"));
        assert!(!is_safe_selector("div[data-ad"));
        assert!(!is_safe_selector("div:not(.ad])"));
        assert!(!is_safe_selector(".ad)"));
    }

    #[test]
    fn hiding_css_has_one_rule_per_selector() {
        let css = hiding_css(&[".a".into(), "#b".into()]);
        assert_eq!(
            css,
            ".a{display:none!important}\n#b{display:none!important}\n"
        );
    }
}
