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
/// that sets `autoplay`, else `default` (`media.autoplay`).
pub fn autoplay_for(default: Autoplay, sites: &BTreeMap<String, Site>, url: &str) -> Autoplay {
    ion_sites::matching(sites, url)
        .iter()
        .rev()
        .find_map(|(_, site)| site.autoplay)
        .unwrap_or(default)
}

/// Whether the engine's `playbackRequiresUserGesture` should be on.
pub fn requires_gesture(policy: Autoplay) -> bool {
    policy != Autoplay::Allow
}

/// Page script (main world, document creation, every frame) for
/// `autoplay = "block"`: until the frame has had a click or key press,
/// `play()` is refused and media that starts anyway (the `autoplay`
/// attribute) is paused.
pub const BLOCK_SCRIPT: &str = r#"(() => {
  if (window.__ionAutoplayBlocked) return;
  window.__ionAutoplayBlocked = true;
  const active = () => !!(navigator.userActivation && navigator.userActivation.hasBeenActive);
  const play = HTMLMediaElement.prototype.play;
  HTMLMediaElement.prototype.play = function () {
    if (!active())
      return Promise.reject(new DOMException("Autoplay is blocked by Ion", "NotAllowedError"));
    return play.apply(this, arguments);
  };
  document.addEventListener("play", event => {
    if (!active() && event.target instanceof HTMLMediaElement)
      event.target.pause();
  }, true);
})();"#;

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
    fn only_allow_lifts_the_gesture_requirement() {
        assert!(!requires_gesture(Autoplay::Allow));
        assert!(requires_gesture(Autoplay::Muted));
        assert!(requires_gesture(Autoplay::Block));
    }

    #[test]
    fn block_script_guards_against_running_twice() {
        assert!(BLOCK_SCRIPT.contains("__ionAutoplayBlocked"));
        assert!(BLOCK_SCRIPT.contains("hasBeenActive"));
    }
}
