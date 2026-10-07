//! Page dialogs: `alert()`, `confirm()`, `prompt()`, "leave this page?" and
//! HTTP sign-in, worded for Ion's own themed dialog.

/// A JavaScript dialog, mirroring `JavaScriptDialogRequest.DialogType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Alert,
    Confirm,
    Prompt,
    BeforeUnload,
}

impl Kind {
    /// From `JavaScriptDialogRequest.DialogType`'s integer value.
    pub fn from_qt(value: i32) -> Option<Kind> {
        Some(match value {
            0 => Kind::Alert,
            1 => Kind::Confirm,
            2 => Kind::Prompt,
            3 => Kind::BeforeUnload,
            _ => return None,
        })
    }

    /// Label of the button that accepts the dialog.
    pub fn accept_label(self) -> &'static str {
        match self {
            Kind::BeforeUnload => "Leave",
            _ => "OK",
        }
    }

    /// Label of the button that dismisses it, or `None` when there is only
    /// one button (`alert()`).
    pub fn reject_label(self) -> Option<&'static str> {
        match self {
            Kind::Alert => None,
            Kind::Confirm | Kind::Prompt => Some("Cancel"),
            Kind::BeforeUnload => Some("Stay"),
        }
    }
}

/// The dialog's heading: "example.com says", or "Leave this page?" for the
/// unsaved-changes check. Pages cannot choose it, so it always names where
/// the message really comes from.
pub fn heading(kind: Kind, origin: &str) -> String {
    match kind {
        Kind::BeforeUnload => "Leave this page?".to_owned(),
        _ => match crate::display_host(origin) {
            Some(host) => format!("{host} says"),
            None => "This page says".to_owned(),
        },
    }
}

/// The body for the unsaved-changes check; browsers ignore the page's own
/// text there so it cannot be used to trap people.
pub const BEFORE_UNLOAD_MESSAGE: &str = "Changes you made may not be saved.";

/// How many dialogs a page may open in a row before Ion offers to block the
/// rest until it navigates.
pub const BLOCK_OFFER_AFTER: u32 = 2;

/// Whether to show "Don't let this page open more dialogs" on the
/// `count`-th dialog (1-based) since the page loaded.
pub fn offer_block(count: u32) -> bool {
    count >= BLOCK_OFFER_AFTER
}

/// Heading of the HTTP sign-in dialog: "Sign in to example.com" or, for a
/// proxy, "Sign in to proxy proxy.example".
pub fn auth_heading(proxy: bool, url: &str, proxy_host: &str) -> String {
    if proxy {
        let host = if proxy_host.is_empty() {
            "the proxy"
        } else {
            proxy_host
        };
        return format!("Sign in to proxy {host}");
    }
    match crate::display_host(url) {
        Some(host) => format!("Sign in to {host}"),
        None => "Sign in".to_owned(),
    }
}

/// The line under the sign-in heading: the site's realm, and a warning when
/// the password would travel unencrypted.
pub fn auth_detail(realm: &str, url: &str, proxy: bool) -> String {
    let mut lines = Vec::new();
    let realm = realm.trim();
    if !realm.is_empty() {
        lines.push(format!("The site says: “{realm}”"));
    }
    let insecure = !proxy && url.starts_with("http://");
    if insecure {
        lines.push("Your connection to this site is not private.".to_owned());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_qt_matches_the_engine_enum() {
        assert_eq!(Kind::from_qt(0), Some(Kind::Alert));
        assert_eq!(Kind::from_qt(1), Some(Kind::Confirm));
        assert_eq!(Kind::from_qt(2), Some(Kind::Prompt));
        assert_eq!(Kind::from_qt(3), Some(Kind::BeforeUnload));
        assert_eq!(Kind::from_qt(4), None);
        assert_eq!(Kind::from_qt(-1), None);
    }

    #[test]
    fn buttons_fit_the_dialog() {
        assert_eq!(Kind::Alert.accept_label(), "OK");
        assert_eq!(Kind::Alert.reject_label(), None);
        assert_eq!(Kind::Confirm.reject_label(), Some("Cancel"));
        assert_eq!(Kind::Prompt.reject_label(), Some("Cancel"));
        assert_eq!(Kind::BeforeUnload.accept_label(), "Leave");
        assert_eq!(Kind::BeforeUnload.reject_label(), Some("Stay"));
    }

    #[test]
    fn heading_names_the_real_origin() {
        assert_eq!(
            heading(Kind::Alert, "https://www.example.com/page"),
            "example.com says"
        );
        assert_eq!(
            heading(Kind::Prompt, "file:///tmp/a.html"),
            "This page says"
        );
        assert_eq!(
            heading(Kind::BeforeUnload, "https://docs.example"),
            "Leave this page?"
        );
    }

    #[test]
    fn block_offer_starts_on_the_second_dialog() {
        assert!(!offer_block(1));
        assert!(offer_block(2));
        assert!(offer_block(10));
    }

    #[test]
    fn auth_heading_for_sites_and_proxies() {
        assert_eq!(
            auth_heading(false, "https://intranet.example/x", ""),
            "Sign in to intranet.example"
        );
        assert_eq!(
            auth_heading(true, "https://a.example", "proxy.corp"),
            "Sign in to proxy proxy.corp"
        );
        assert_eq!(auth_heading(true, "", ""), "Sign in to proxy the proxy");
        assert_eq!(auth_heading(false, "about:blank", ""), "Sign in");
    }

    #[test]
    fn auth_detail_quotes_realm_and_warns_on_http() {
        assert_eq!(
            auth_detail("Staff only", "https://a.example", false),
            "The site says: “Staff only”"
        );
        assert_eq!(
            auth_detail(" ", "http://a.example", false),
            "Your connection to this site is not private."
        );
        assert_eq!(
            auth_detail("R", "http://a.example", false),
            "The site says: “R”\nYour connection to this site is not private."
        );
        assert_eq!(auth_detail("", "http://a.example", true), "");
    }
}
