//! The everyday browser features that decide whether Ion can be a daily
//! driver: downloads, per-site zoom, permission prompts, find in page, the
//! page context menu and the new-tab page.
//!
//! Like every `ion-*` crate this is Qt-free. `ion-app` adapts it to QML in
//! `bridge/downloads.rs`, `bridge/zoom.rs` and `bridge/page.rs`.

pub mod context_menu;
pub mod downloads;
pub mod find;
pub mod new_tab;
pub mod permissions;
pub mod zoom;

/// The host part of a URL as people read it (`www.` dropped), or `None` for
/// URLs without one (`about:blank`, `file://`, `data:`).
pub fn display_host(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_owned();
    (!host.is_empty()).then_some(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_host_drops_www_and_lowercases() {
        assert_eq!(
            display_host("https://WWW.Example.com/a?b").as_deref(),
            Some("example.com")
        );
        assert_eq!(
            display_host("http://localhost:8080/").as_deref(),
            Some("localhost")
        );
    }

    #[test]
    fn display_host_is_none_without_a_host() {
        assert_eq!(display_host("about:blank"), None);
        assert_eq!(display_host("file:///tmp/x.html"), None);
        assert_eq!(display_host("not a url"), None);
    }
}
