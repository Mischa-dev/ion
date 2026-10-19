//! Media sessions: what each tab is playing, for Ion's now-playing controls
//! and the system's media controls (MPRIS on Linux).
//!
//! [`PAGE_SCRIPT`] runs in every frame of every page. It watches the page's
//! audio and video and its Media Session (`navigator.mediaSession`: title,
//! artist, artwork, next/previous handlers) and reports changes to Ion as
//! one console message, [`MESSAGE`], a frame id, a space and JSON, which
//! [`parse`] reads.
//! Ion answers with [`action_script`], run in each frame.

/// What [`PAGE_SCRIPT`] logs before its JSON.
pub const MESSAGE: &str = "\u{2063}ion-media ";

/// A tab's media, as its page reports it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct State {
    pub playing: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// The largest artwork image, as an absolute URL.
    pub artwork: String,
    /// Seconds; 0 when unknown or live.
    pub duration: f64,
    pub position: f64,
    pub has_video: bool,
    /// Whether the page handles "previous track" / "next track".
    pub can_previous: bool,
    pub can_next: bool,
}

impl State {
    /// The title to show: the page's own, else the tab's.
    pub fn display_title<'a>(&'a self, tab_title: &'a str) -> &'a str {
        if self.title.trim().is_empty() {
            tab_title
        } else {
            &self.title
        }
    }

    /// `"Playing"` or `"Paused"`, as MPRIS spells it.
    pub fn playback_status(&self) -> &'static str {
        if self.playing { "Playing" } else { "Paused" }
    }
}

/// Reads a report: the console message's JSON, after [`MESSAGE`] and the
/// frame id (see [`split`]). `None` for
/// anything else, or a frame with nothing playable any more.
pub fn parse(json: &str) -> Option<State> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let text = |key: &str, max: usize| -> String {
        value[key]
            .as_str()
            .unwrap_or("")
            .chars()
            .filter(|c| !c.is_control())
            .take(max)
            .collect::<String>()
            .trim()
            .to_owned()
    };
    let seconds = |key: &str| {
        value[key]
            .as_f64()
            .filter(|s| s.is_finite() && *s >= 0.0)
            .unwrap_or(0.0)
    };
    if value["gone"].as_bool() == Some(true) {
        return None;
    }
    let artwork = text("artwork", 2048);
    let artwork = if artwork.starts_with("https://")
        || artwork.starts_with("http://")
        || artwork.starts_with("data:image/")
    {
        artwork
    } else {
        String::new()
    };
    let duration = seconds("duration");
    Some(State {
        playing: value["playing"].as_bool().unwrap_or(false),
        title: text("title", 300),
        artist: text("artist", 300),
        album: text("album", 300),
        artwork,
        duration,
        position: if duration > 0.0 {
            seconds("position").min(duration)
        } else {
            0.0
        },
        has_video: value["video"].as_bool().unwrap_or(false),
        can_previous: value["previous"].as_bool().unwrap_or(false),
        can_next: value["next"].as_bool().unwrap_or(false),
    })
}

/// Splits a console message into the reporting frame's id and its JSON, or
/// `None` when it isn't a report.
pub fn split(message: &str) -> Option<(&str, &str)> {
    message.strip_prefix(MESSAGE)?.split_once(' ')
}

/// An action Ion asks a page to take.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
    /// Move by this many seconds.
    SeekBy(f64),
    /// Go to this many seconds in.
    SeekTo(f64),
}

impl Action {
    /// Reads an action name as QML and MPRIS give it: `"play"`, `"pause"`,
    /// `"playpause"`, `"stop"`, `"next"`, `"previous"`, `"seekby"`,
    /// `"seekto"` (the last two use `value`).
    pub fn from_name(name: &str, value: f64) -> Option<Self> {
        let value = if value.is_finite() { value } else { 0.0 };
        Some(match name {
            "play" => Self::Play,
            "pause" => Self::Pause,
            "playpause" => Self::PlayPause,
            "stop" => Self::Stop,
            "next" => Self::Next,
            "previous" => Self::Previous,
            "seekby" => Self::SeekBy(value),
            "seekto" => Self::SeekTo(value.max(0.0)),
            _ => return None,
        })
    }
}

/// Script (Ion's world, each frame) that hands `action` to the page script
/// of the frame with id `frame` (as [`split`] gave it), which uses the page's
/// own Media Session handler when it has one. Other frames ignore it.
pub fn action_script(action: Action, frame: &str) -> String {
    // Frame ids are the page script's own, but a page could log any text.
    let frame: String = frame.chars().filter(char::is_ascii_alphanumeric).collect();
    let (name, value) = match action {
        Action::Play => ("play", 0.0),
        Action::Pause => ("pause", 0.0),
        Action::PlayPause => ("playpause", 0.0),
        Action::Stop => ("stop", 0.0),
        Action::Next => ("nexttrack", 0.0),
        Action::Previous => ("previoustrack", 0.0),
        Action::SeekBy(s) => ("seekby", s),
        Action::SeekTo(s) => ("seekto", s),
    };
    format!(
        "document.dispatchEvent(new CustomEvent(\"ion-media-action\", {{ detail: \"{name} {value} {frame}\" }}));"
    )
}

/// Page script (main world, document creation, every frame).
pub const PAGE_SCRIPT: &str = r#"(() => {
  if (window.__ionMediaSession) return;
  window.__ionMediaSession = true;
  const frame = Math.random().toString(36).slice(2, 10);
  const log = console.debug.bind(console);
  const handlers = new Map();
  const session = navigator.mediaSession;
  if (session && session.setActionHandler) {
    const set = session.setActionHandler.bind(session);
    session.setActionHandler = (action, handler) => {
      if (handler) handlers.set(action, handler); else handlers.delete(action);
      try { set(action, handler); } catch (e) { if (!handler) return; throw e; }
      report();
    };
    const proto = Object.getPrototypeOf(session);
    const meta = Object.getOwnPropertyDescriptor(proto, "metadata");
    if (meta && meta.set) {
      Object.defineProperty(session, "metadata", {
        configurable: true,
        get: () => meta.get.call(session),
        set: value => { meta.set.call(session, value); report(); },
      });
    }
  }
  // The media people care about: playing beats paused, sound beats silence,
  // bigger beats smaller.
  const pick = () => {
    let best = null, bestScore = 0;
    for (const el of document.querySelectorAll("video, audio")) {
      if (!el.currentSrc && !el.srcObject) continue;
      const box = el.tagName === "VIDEO" ? el.getBoundingClientRect() : { width: 1, height: 1 };
      const score = (el.paused ? 1 : 1e6) * (el.muted ? 1 : 10) * (1 + box.width * box.height / 1e5)
        * (el.readyState > 0 ? 1 : 0.1) * (el.ended ? 0.1 : 1);
      if (score > bestScore) { best = el; bestScore = score; }
    }
    return best;
  };
  let last = "", timer = 0, reported = false;
  const report = () => {
    if (timer) return;
    timer = setTimeout(() => {
      timer = 0;
      const el = pick();
      const md = session && session.metadata;
      let state = null;
      if (!el) {
        if (reported) state = { gone: true };
      } else if (!el.paused || reported) {
        let artwork = "", size = 0;
        for (const art of (md && md.artwork) || []) {
          const s = parseInt(String(art.sizes || "0").split("x")[0], 10) || 1;
          if (s >= size) { size = s; try { artwork = new URL(art.src, location.href).href; } catch (e) {} }
        }
        state = {
          playing: !el.paused && !el.ended,
          title: (md && md.title) || "", artist: (md && md.artist) || "", album: (md && md.album) || "",
          artwork, duration: isFinite(el.duration) ? el.duration : 0, position: el.currentTime || 0,
          video: el.tagName === "VIDEO" && el.videoWidth > 0,
          previous: handlers.has("previoustrack"), next: handlers.has("nexttrack"),
        };
      }
      if (!state) return;
      const json = JSON.stringify(state);
      if (json === last) return;
      last = json;
      reported = !state.gone;
      log("MESSAGE" + frame + " " + json);
    }, 150);
  };
  for (const type of ["play", "playing", "pause", "ended", "emptied", "loadedmetadata", "durationchange", "seeked", "volumechange"])
    document.addEventListener(type, report, true);
  setInterval(() => { if (reported) report(); }, 5000);
  document.addEventListener("ion-media-action", event => {
    const [action, raw, target] = String(event.detail).split(" ");
    const value = Number(raw) || 0;
    // Only the frame whose media Ion is showing; others keep still.
    if (!reported || target !== frame) return;
    const el = pick();
    const call = (name, details) => {
      const handler = handlers.get(name);
      if (!handler) return false;
      try { handler(Object.assign({ action: name }, details || {})); } catch (e) {}
      return true;
    };
    switch (action) {
      case "play": if (!call("play") && el) el.play(); break;
      case "pause": if (!call("pause") && el) el.pause(); break;
      case "playpause":
        if (el && el.paused) { if (!call("play")) el.play(); }
        else if (!call("pause") && el) el.pause();
        break;
      case "stop": if (!call("stop") && el) el.pause(); break;
      case "nexttrack": call("nexttrack"); break;
      case "previoustrack": if (!call("previoustrack") && el) el.currentTime = 0; break;
      case "seekby":
        if (!call(value < 0 ? "seekbackward" : "seekforward", { seekOffset: Math.abs(value) }) && el)
          el.currentTime = Math.max(0, el.currentTime + value);
        break;
      case "seekto": if (!call("seekto", { seekTime: value }) && el) el.currentTime = value; break;
    }
  });
})();"#;

/// The page script with its message filled in.
pub fn page_script() -> String {
    PAGE_SCRIPT.replace("\"MESSAGE\"", &crate::pip::js_string(MESSAGE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_are_read_and_cleaned() {
        let state = parse(
            r#"{"playing":true,"title":" Song\u0007 ","artist":"Band","artwork":"javascript:alert(1)",
                "duration":200,"position":250,"video":false,"next":true}"#,
        )
        .unwrap();
        assert!(state.playing);
        assert_eq!(state.title, "Song");
        assert_eq!(state.artwork, "");
        assert_eq!(state.position, 200.0);
        assert!(state.can_next && !state.can_previous);
        assert_eq!(state.playback_status(), "Playing");
        assert_eq!(parse(r#"{"gone":true}"#), None);
        assert_eq!(parse("not json"), None);
    }

    #[test]
    fn live_streams_have_no_position() {
        let state = parse(r#"{"playing":true,"duration":-1,"position":42}"#).unwrap();
        assert_eq!((state.duration, state.position), (0.0, 0.0));
        assert_eq!(state.display_title("Tab"), "Tab");
    }

    #[test]
    fn actions_round_trip_to_the_page() {
        assert_eq!(
            Action::from_name("seekby", -10.0),
            Some(Action::SeekBy(-10.0))
        );
        assert_eq!(Action::from_name("seekto", -3.0), Some(Action::SeekTo(0.0)));
        assert_eq!(Action::from_name("dance", 0.0), None);
        assert!(action_script(Action::Next, "ab12").contains("\"nexttrack 0 ab12\""));
        assert!(action_script(Action::SeekBy(-10.0), "ab12").contains("\"seekby -10 ab12\""));
        // A forged frame id can't break out of the string.
        assert!(action_script(Action::Play, "x\"); alert(1); //").contains("\"play 0 xalert1\""));
    }

    #[test]
    fn page_script_logs_ions_message() {
        assert!(page_script().contains(&format!("log(\"{MESSAGE}\" + frame")));
        let message = format!("{MESSAGE}ab12 {{\"playing\":true}}");
        assert_eq!(split(&message), Some(("ab12", "{\"playing\":true}")));
        assert_eq!(split("hello"), None);
    }
}
