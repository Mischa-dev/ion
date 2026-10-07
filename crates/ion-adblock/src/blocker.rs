//! The compiled filter engine.

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
    /// or a hosts file. Unparseable lines are skipped. Only network rules are
    /// kept: Ion doesn't do cosmetic filtering, so element-hiding rules would
    /// only cost memory.
    pub fn from_lists<'a>(lists: impl IntoIterator<Item = &'a str>) -> Self {
        let mut set = FilterSet::new(false);
        for text in lists {
            let options = ParseOptions {
                format: format_of(text),
                rule_types: RuleTypes::NetworkOnly,
                ..ParseOptions::default()
            };
            set.add_filter_list(text.to_owned(), options);
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
}
