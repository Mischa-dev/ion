//! Inline completion for the URL bar: typing `git` fills in `hub.com/` from
//! a page you have open or visited, with the filled part selected so the next
//! keystroke replaces it.
//!
//! Only http(s) pages are used. Without a `/` in the input the completion is
//! the site (`github.com/`); with one it completes the next path segment
//! (`github.com/Mischa-dev/`), so it never jumps further than you'd expect.

/// The completed text for `input`, or `None` when nothing fits. The result
/// starts with `input` exactly as typed and is strictly longer. `urls` are
/// tried in order, so pass the most likely pages first.
pub fn complete<'a>(input: &str, urls: impl IntoIterator<Item = &'a str>) -> Option<String> {
    if input.is_empty()
        || !input.is_ascii()
        || input.starts_with('!')
        || input.contains(char::is_whitespace)
    {
        return None;
    }
    let typed = input.to_ascii_lowercase();
    let typed = typed
        .strip_prefix("https://")
        .or_else(|| typed.strip_prefix("http://"))
        .unwrap_or(&typed);
    if typed.is_empty() {
        return None;
    }

    urls.into_iter().find_map(|url| {
        let rest = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))?;
        let bare = rest.strip_prefix("www.");
        [Some(rest), bare]
            .into_iter()
            .flatten()
            .find_map(|candidate| completion(typed, candidate))
            .map(|tail| format!("{input}{tail}"))
    })
}

/// The text to append to `typed` (lowercase) so it reads as the start of
/// `candidate`, up to and including the next `/`.
fn completion<'a>(typed: &str, candidate: &'a str) -> Option<&'a str> {
    if candidate.len() <= typed.len()
        || !candidate.is_char_boundary(typed.len())
        || !candidate[..typed.len()].eq_ignore_ascii_case(typed)
    {
        return None;
    }
    let tail = &candidate[typed.len()..];
    // A site completes to "host/"; a path one segment at a time.
    let end = tail.find('/').map_or(tail.len(), |slash| slash + 1);
    let tail = &tail[..end];
    // Stop before a query or fragment; nobody wants those filled in.
    let tail = tail.split(['?', '#']).next().unwrap_or_default();
    (!tail.is_empty()).then_some(tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    const URLS: &[&str] = &[
        "https://github.com/Mischa-dev/ion/pulls",
        "https://www.rust-lang.org/learn",
        "http://example.com",
        "file:///home/me/notes.html",
    ];

    fn fill(input: &str) -> Option<String> {
        complete(input, URLS.iter().copied())
    }

    #[test]
    fn completes_the_site_first() {
        assert_eq!(fill("git").as_deref(), Some("github.com/"));
        assert_eq!(fill("GIT").as_deref(), Some("GIThub.com/"));
        assert_eq!(fill("rust").as_deref(), Some("rust-lang.org/"));
        assert_eq!(fill("www.ru").as_deref(), Some("www.rust-lang.org/"));
        assert_eq!(fill("exa").as_deref(), Some("example.com"));
    }

    #[test]
    fn completes_one_path_segment_at_a_time() {
        assert_eq!(
            fill("github.com/").as_deref(),
            Some("github.com/Mischa-dev/")
        );
        assert_eq!(
            fill("github.com/Mischa-dev/ion/p").as_deref(),
            Some("github.com/Mischa-dev/ion/pulls")
        );
    }

    #[test]
    fn keeps_a_typed_scheme() {
        assert_eq!(fill("https://git").as_deref(), Some("https://github.com/"));
    }

    #[test]
    fn leaves_searches_bangs_and_other_pages_alone() {
        assert_eq!(fill(""), None);
        assert_eq!(fill("git hub"), None);
        assert_eq!(fill("!gh"), None);
        assert_eq!(fill("home"), None);
        assert_eq!(fill("github.com/Mischa-dev/ion/pulls"), None);
        assert_eq!(fill("https://"), None);
        assert_eq!(fill("zzz"), None);
    }
}
