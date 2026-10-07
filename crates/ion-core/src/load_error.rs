//! What Ion's own error page says when a page fails to load.
//!
//! QtWebEngine reports a failed load as an error domain plus a code (a
//! Chromium net error for most domains). This turns that into a short title
//! and a sentence a person can act on, or `None` when no error page should
//! show: a load the person stopped, or an HTTP status the server sent a page
//! for.

/// Where a load error came from, as `WebEngineLoadingInfo.errorDomain`
/// numbers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorDomain {
    Internal,
    Connection,
    Certificate,
    /// Chromium's HTTP-layer net errors (no response to show).
    Http,
    Ftp,
    Dns,
    /// A 4xx or 5xx status; the server's own page shows instead.
    HttpStatus,
    Other,
}

impl ErrorDomain {
    /// The domain for QtWebEngine's `ErrorDomain` value.
    pub fn from_qt(value: i32) -> Self {
        match value {
            1 => Self::Internal,
            2 => Self::Connection,
            3 => Self::Certificate,
            4 => Self::Http,
            5 => Self::Ftp,
            6 => Self::Dns,
            7 => Self::HttpStatus,
            _ => Self::Other,
        }
    }
}

/// The error page's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadError {
    pub title: String,
    pub detail: String,
}

// Chromium net error codes (net/base/net_error_list.h), without the sign.
const ABORTED: i32 = 3;
const TIMED_OUT: i32 = 7;
const CONNECTION_CLOSED: i32 = 100;
const CONNECTION_RESET: i32 = 101;
const CONNECTION_REFUSED: i32 = 102;
const NAME_NOT_RESOLVED: i32 = 105;
const INTERNET_DISCONNECTED: i32 = 106;
const ADDRESS_UNREACHABLE: i32 = 109;
const CONNECTION_TIMED_OUT: i32 = 118;
const NAME_RESOLUTION_FAILED: i32 = 137;
const BLOCKED_BY_CLIENT: i32 = 20;
const UNSAFE_PORT: i32 = 312;

/// The error page for a failed load of `url`, or `None` when none should
/// show. `message` is QtWebEngine's own description, used when Ion has
/// nothing better to say.
pub fn describe(domain: ErrorDomain, code: i32, url: &str, message: &str) -> Option<LoadError> {
    let code = code.abs();
    // Stopped or replaced by another navigation; HTTP statuses come with the
    // server's own page.
    if code == ABORTED || domain == ErrorDomain::HttpStatus {
        return None;
    }
    let site = site_name(url);
    let page = |title: String, detail: &str| {
        Some(LoadError {
            title,
            detail: detail.to_owned(),
        })
    };
    if domain == ErrorDomain::Certificate {
        return page(
            format!("{site} can't be trusted"),
            "Its security certificate isn't valid, so someone could be pretending to be \
             this site. Ion stopped loading it.",
        );
    }
    match code {
        INTERNET_DISCONNECTED => page(
            "You're offline".into(),
            "Ion can't reach the internet. Check your connection and try again.",
        ),
        NAME_NOT_RESOLVED | NAME_RESOLUTION_FAILED => page(
            format!("Can't find {site}"),
            "Check the address for typos. If it's right, your connection or DNS may be down.",
        ),
        CONNECTION_REFUSED => page(
            format!("{site} refused to connect"),
            "The site may be down or not accepting connections right now.",
        ),
        TIMED_OUT | CONNECTION_TIMED_OUT => page(
            format!("{site} took too long to respond"),
            "The site may be busy or down. Try again in a moment.",
        ),
        CONNECTION_CLOSED | CONNECTION_RESET | ADDRESS_UNREACHABLE => page(
            format!("The connection to {site} was lost"),
            "Try again. If it keeps happening, check your connection.",
        ),
        UNSAFE_PORT => page(
            format!("Ion won't connect to {site} on that port"),
            "Browsers refuse this port because other kinds of servers use it.",
        ),
        BLOCKED_BY_CLIENT => page(
            format!("{site} was blocked"),
            "Ion's content blocker stopped this page from loading.",
        ),
        _ => {
            let detail = if message.trim().is_empty() {
                "Something went wrong while loading it."
            } else {
                message.trim()
            };
            page(format!("Can't open {site}"), detail)
        }
    }
}

/// The host of `url`, or the whole URL when it has none (a file, say).
fn site_name(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(parsed) => match parsed.host_str() {
            Some(host) if !host.is_empty() => host.trim_end_matches('.').to_owned(),
            _ => url.to_owned(),
        },
        Err(_) if url.is_empty() => "this page".to_owned(),
        Err(_) => url.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn title(domain: ErrorDomain, code: i32, url: &str) -> Option<String> {
        describe(domain, code, url, "net::ERR_SOMETHING").map(|e| e.title)
    }

    #[test]
    fn names_common_failures_after_the_site() {
        let url = "https://Example.com/a";
        assert_eq!(
            title(ErrorDomain::Dns, -105, url).as_deref(),
            Some("Can't find example.com")
        );
        assert_eq!(
            title(ErrorDomain::Connection, -102, url).as_deref(),
            Some("example.com refused to connect")
        );
        assert_eq!(
            title(ErrorDomain::Connection, 106, url).as_deref(),
            Some("You're offline")
        );
        // Chromium's HTTP-layer errors have no server page to show.
        assert_eq!(
            title(ErrorDomain::Http, -312, url).as_deref(),
            Some("Ion won't connect to example.com on that port")
        );
        assert_eq!(
            title(ErrorDomain::Certificate, -202, url).as_deref(),
            Some("example.com can't be trusted")
        );
    }

    #[test]
    fn no_page_for_stopped_loads_or_http_errors() {
        assert_eq!(title(ErrorDomain::Internal, -3, "https://a.com/"), None);
        assert_eq!(title(ErrorDomain::HttpStatus, 500, "https://a.com/"), None);
    }

    #[test]
    fn unknown_errors_fall_back_to_qts_message() {
        let error = describe(ErrorDomain::Internal, -999, "file:///tmp/x.html", " boom ").unwrap();
        assert_eq!(error.title, "Can't open file:///tmp/x.html");
        assert_eq!(error.detail, "boom");
        assert_eq!(ErrorDomain::from_qt(6), ErrorDomain::Dns);
        assert_eq!(ErrorDomain::from_qt(42), ErrorDomain::Other);
    }
}
