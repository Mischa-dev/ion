//! The compiled filter engine.

use std::fmt;

use adblock::Engine;
use adblock::lists::{FilterSet, ParseOptions};
use adblock::request::Request;

use crate::RequestInfo;

/// Filter lists compiled into an adblock-rust engine, ready to match requests.
pub struct Blocker {
    engine: Engine,
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
    /// Compile the given filter list texts. Unparseable lines are skipped.
    pub fn from_lists<'a>(lists: impl IntoIterator<Item = &'a str>) -> Self {
        let mut set = FilterSet::new(false);
        for text in lists {
            set.add_filter_list(text.to_owned(), ParseOptions::default());
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
        let Ok(req) = Request::new(
            request.url,
            request.first_party,
            request.resource.filter_type(),
            request.method,
        ) else {
            return false;
        };
        self.engine.check_network_request(&req).should_block()
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
