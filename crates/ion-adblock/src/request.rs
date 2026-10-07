//! A network request as the blocker sees it.

/// What kind of resource a request loads. Mirrors the cases of
/// `QWebEngineUrlRequestInfo::ResourceType` that matter for filter matching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceType {
    MainFrame,
    SubFrame,
    Stylesheet,
    Script,
    Image,
    Font,
    Object,
    Media,
    Xhr,
    Ping,
    WebSocket,
    CspReport,
    Other,
}

impl ResourceType {
    /// Map a `QWebEngineUrlRequestInfo::ResourceType` value (Qt 6.11).
    /// Workers load scripts, favicons are images, and navigation preloads
    /// are the frames they belong to.
    pub fn from_webengine(code: i32) -> Self {
        match code {
            0 | 19 => Self::MainFrame,
            1 | 20 => Self::SubFrame,
            2 => Self::Stylesheet,
            3 | 9 | 10 | 15 => Self::Script,
            4 | 12 => Self::Image,
            5 => Self::Font,
            7 | 17 => Self::Object,
            8 => Self::Media,
            13 | 21 => Self::Xhr,
            14 => Self::Ping,
            16 => Self::CspReport,
            254 => Self::WebSocket,
            _ => Self::Other,
        }
    }

    /// The request type name adblock-rust filters match against.
    pub fn filter_type(self) -> &'static str {
        match self {
            Self::MainFrame => "document",
            Self::SubFrame => "subdocument",
            Self::Stylesheet => "stylesheet",
            Self::Script => "script",
            Self::Image => "image",
            Self::Font => "font",
            Self::Object => "object",
            Self::Media => "media",
            Self::Xhr => "xmlhttprequest",
            Self::Ping => "ping",
            Self::WebSocket => "websocket",
            Self::CspReport => "csp_report",
            Self::Other => "other",
        }
    }
}

/// One request QtWebEngine is about to make.
#[derive(Clone, Copy, Debug)]
pub struct RequestInfo<'a> {
    /// The URL being requested.
    pub url: &'a str,
    /// The URL of the page (top-level frame) that made the request.
    pub first_party: &'a str,
    pub resource: ResourceType,
    /// HTTP method, e.g. `GET`.
    pub method: &'a str,
}

impl<'a> RequestInfo<'a> {
    pub fn new(url: &'a str, first_party: &'a str, resource: ResourceType) -> Self {
        Self {
            url,
            first_party,
            resource,
            method: "GET",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_webengine_resource_types() {
        assert_eq!(ResourceType::from_webengine(0), ResourceType::MainFrame);
        assert_eq!(ResourceType::from_webengine(3), ResourceType::Script);
        assert_eq!(ResourceType::from_webengine(10), ResourceType::Script);
        assert_eq!(ResourceType::from_webengine(12), ResourceType::Image);
        assert_eq!(ResourceType::from_webengine(13), ResourceType::Xhr);
        assert_eq!(ResourceType::from_webengine(19), ResourceType::MainFrame);
        assert_eq!(ResourceType::from_webengine(254), ResourceType::WebSocket);
        assert_eq!(ResourceType::from_webengine(16), ResourceType::CspReport);
        assert_eq!(ResourceType::CspReport.filter_type(), "csp_report");
        assert_eq!(ResourceType::from_webengine(6), ResourceType::Other);
        assert_eq!(ResourceType::from_webengine(255), ResourceType::Other);
    }
}
