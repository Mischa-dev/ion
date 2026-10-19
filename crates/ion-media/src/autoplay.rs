//! Autoplay: which media a page may start before you interact with it.
//!
//! QtWebEngine has one switch, `playbackRequiresUserGesture`. On, it applies
//! Chromium's policy (silent media may play, sound waits for a click or
//! key); off, anything plays. `"block"` goes further than the engine can:
//! [`BLOCK_SCRIPT`] pauses even silent media until the page is interacted
//! with.

use std::collections::BTreeMap;

use ion_config::{Autoplay, Site};

/// The autoplay policy for pages at `url`: the most specific `[sites]` entry
/// that sets `autoplay`, else `default` (`media.autoplay`). A `blob:` or
/// `filesystem:` URL counts as the site that made it.
pub fn autoplay_for(default: Autoplay, sites: &BTreeMap<String, Site>, url: &str) -> Autoplay {
    ion_sites::matching(sites, creator(url))
        .iter()
        .rev()
        .find_map(|(_, site)| site.autoplay)
        .unwrap_or(default)
}

/// The URL a `blob:` or `filesystem:` URL wraps; other URLs as they are.
fn creator(url: &str) -> &str {
    let url = url.trim();
    for scheme in ["blob:", "filesystem:"] {
        if url.len() > scheme.len() && url[..scheme.len()].eq_ignore_ascii_case(scheme) {
            return &url[scheme.len()..];
        }
    }
    url
}

/// Whether the engine's `playbackRequiresUserGesture` should be on.
pub fn requires_gesture(policy: Autoplay) -> bool {
    policy != Autoplay::Allow
}

/// Page script (main world, document creation, every frame) for
/// `autoplay = "block"`: until the frame has had a click or key press,
/// `play()` is refused and media that starts anyway (the `autoplay`
/// attribute, in shadow roots too) is paused. Run again on a loaded page (the policy changed to
/// "block"), it also pauses what is already playing on its own.
/// [`UNBLOCK_SCRIPT`] lifts it from a loaded page.
pub const BLOCK_SCRIPT: &str = r#"(() => {
  const active = () => !window.__ionAutoplayBlocked ||
    !!(navigator.userActivation && navigator.userActivation.hasBeenActive);
  // Every element under `root`, inside open shadow roots too.
  const walk = (root, visit) => root.querySelectorAll("*").forEach(element => {
    visit(element);
    if (element.shadowRoot) walk(element.shadowRoot, visit);
  });
  window.__ionAutoplayBlocked = true;
  if (!active())
    walk(document, element => { if (element instanceof HTMLMediaElement) element.pause(); });
  if (window.__ionAutoplayInstalled) return;
  window.__ionAutoplayInstalled = true;
  const play = HTMLMediaElement.prototype.play;
  HTMLMediaElement.prototype.play = function () {
    if (!active())
      return Promise.reject(new DOMException("Autoplay is blocked by Ion", "NotAllowedError"));
    return play.apply(this, arguments);
  };
  const guard = event => {
    if (!active() && event.target instanceof HTMLMediaElement)
      event.target.pause();
  };
  // "play" doesn't leave a shadow root, so each one listens as well: those
  // made from now on (closed ones too), and open ones already there.
  document.addEventListener("play", guard, true);
  const attachShadow = Element.prototype.attachShadow;
  Element.prototype.attachShadow = function () {
    const root = attachShadow.apply(this, arguments);
    root.addEventListener("play", guard, true);
    return root;
  };
  walk(document, element => {
    if (element.shadowRoot) element.shadowRoot.addEventListener("play", guard, true);
  });
})();"#;

/// Page script (main world) that lifts [`BLOCK_SCRIPT`] from a loaded page
/// whose policy is no longer "block".
pub const UNBLOCK_SCRIPT: &str = "window.__ionAutoplayBlocked = false;";

#[cfg(test)]
mod tests {
    use super::*;

    fn sites(entries: &[(&str, Option<Autoplay>)]) -> BTreeMap<String, Site> {
        entries
            .iter()
            .map(|(host, autoplay)| {
                (
                    (*host).to_owned(),
                    Site {
                        autoplay: *autoplay,
                        ..Site::default()
                    },
                )
            })
            .collect()
    }

    #[test]
    fn most_specific_site_wins() {
        let s = sites(&[
            ("*", Some(Autoplay::Block)),
            ("youtube.com", Some(Autoplay::Allow)),
            ("music.youtube.com", None),
        ]);
        assert_eq!(
            autoplay_for(Autoplay::Muted, &s, "https://music.youtube.com/watch"),
            Autoplay::Allow
        );
        assert_eq!(
            autoplay_for(Autoplay::Muted, &s, "https://news.example/"),
            Autoplay::Block
        );
        assert_eq!(
            autoplay_for(Autoplay::Muted, &BTreeMap::new(), "https://news.example/"),
            Autoplay::Muted
        );
    }

    #[test]
    fn blob_urls_count_as_their_site() {
        let s = sites(&[("music.example", Some(Autoplay::Block))]);
        assert_eq!(
            autoplay_for(Autoplay::Allow, &s, "blob:https://music.example/1234"),
            Autoplay::Block
        );
        assert_eq!(creator("https://a.example/"), "https://a.example/");
    }

    #[test]
    fn only_allow_lifts_the_gesture_requirement() {
        assert!(!requires_gesture(Autoplay::Allow));
        assert!(requires_gesture(Autoplay::Muted));
        assert!(requires_gesture(Autoplay::Block));
    }

    #[test]
    fn block_script_guards_against_running_twice() {
        assert!(BLOCK_SCRIPT.contains("if (window.__ionAutoplayInstalled) return;"));
        assert!(BLOCK_SCRIPT.contains("hasBeenActive"));
    }

    #[test]
    fn block_script_reaches_into_shadow_roots() {
        assert!(BLOCK_SCRIPT.contains("Element.prototype.attachShadow"));
        assert!(BLOCK_SCRIPT.contains("element.shadowRoot"));
    }
}
