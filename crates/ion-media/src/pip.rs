//! Picture-in-picture.
//!
//! QtWebEngine has no picture-in-picture window of its own, so Ion makes one:
//! the tab's whole web view moves into a small window, and these scripts
//! make the chosen video fill that view. They run in Ion's own script world,
//! which shares only the page's DOM.
//!
//! A video inside an iframe is handled frame by frame: the frame holding the
//! video gets [`enter_script`], and each frame above it gets
//! [`frame_script`] for the iframe on the way down.

/// What [`PAGE_SCRIPT`] logs (at debug level) to ask Ion to open or close
/// picture-in-picture. Starts with an invisible separator so no page's own
/// logging matches by accident.
pub const ENTER_MESSAGE: &str = "\u{2063}ion-pip-enter";
pub const EXIT_MESSAGE: &str = "\u{2063}ion-pip-exit";

/// Page script (main world, document creation, every frame): the standard
/// picture-in-picture API, so sites show their own PiP buttons, answered by
/// Ion's window. A request only counts while the person is interacting
/// with the page (a click or key press just now), as in other browsers.
pub const PAGE_SCRIPT: &str = r#"(() => {
  if (window.__ionPip || typeof HTMLVideoElement === "undefined") return;
  window.__ionPip = true;
  const log = console.debug.bind(console);
  const proto = HTMLVideoElement.prototype;
  if (proto.requestPictureInPicture && document.pictureInPictureEnabled) return;
  let current = null;
  const windowFor = video => ({ width: video.clientWidth, height: video.clientHeight, onresize: null,
    addEventListener() {}, removeEventListener() {} });
  Object.defineProperty(document, "pictureInPictureEnabled", { get: () => true, configurable: true });
  Object.defineProperty(document, "pictureInPictureElement", { get: () => current, configurable: true });
  proto.requestPictureInPicture = function () {
    if (this.disablePictureInPicture)
      return Promise.reject(new DOMException("Picture-in-picture is disabled", "InvalidStateError"));
    if (navigator.userActivation && !navigator.userActivation.isActive)
      return Promise.reject(new DOMException("Needs a click", "NotAllowedError"));
    if (current && current !== this) current.dispatchEvent(new Event("leavepictureinpicture"));
    current = this;
    this.setAttribute("data-ion-pip-request", "");
    log("ENTER");
    const pipWindow = windowFor(this);
    const event = new Event("enterpictureinpicture");
    event.pictureInPictureWindow = pipWindow;
    this.dispatchEvent(event);
    return Promise.resolve(pipWindow);
  };
  document.exitPictureInPicture = function () {
    if (!current) return Promise.reject(new DOMException("Not in picture-in-picture", "InvalidStateError"));
    const video = current;
    current = null;
    log("EXIT");
    video.dispatchEvent(new Event("leavepictureinpicture"));
    return Promise.resolve();
  };
  // Ion leaving picture-in-picture by itself (its window closed).
  document.addEventListener("ion-pip-left", () => {
    if (!current) return;
    const video = current;
    current = null;
    video.dispatchEvent(new Event("leavepictureinpicture"));
  });
})();"#;

/// The page script with its messages filled in.
pub fn page_script() -> String {
    PAGE_SCRIPT
        .replace("\"ENTER\"", &js_string(ENTER_MESSAGE))
        .replace("\"EXIT\"", &js_string(EXIT_MESSAGE))
}

/// Run in each frame: the frame's best video as JSON
/// `{ "score": n, "playing": bool }`, or `null` when it has none. Bigger,
/// playing, audible videos score higher.
pub const PICK_SCRIPT: &str = r#"(() => {
  let best = null;
  for (const video of document.querySelectorAll("video")) {
    const box = video.getBoundingClientRect();
    const area = box.width * box.height;
    if (video.readyState < 1 || area < 1) continue;
    const playing = !video.paused && !video.ended;
    const asked = video.hasAttribute("data-ion-pip-request") ? 1000 : 1;
    const score = area * asked * (playing ? 4 : 1) * (video.muted || video.volume === 0 ? 1 : 2);
    if (!best || score > best.score) best = { score, playing };
  }
  return best ? JSON.stringify(best) : "null";
})()"#;

/// The style sheet that makes `[data-ion-pip]` fill the viewport.
const STYLE: &str = "html[data-ion-pip-root], html[data-ion-pip-root] body { overflow: hidden !important; } \
html[data-ion-pip-root] body { visibility: hidden !important; } \
[data-ion-pip] { visibility: visible !important; position: fixed !important; inset: 0 !important; \
width: 100vw !important; height: 100vh !important; max-width: none !important; max-height: none !important; \
min-width: 0 !important; min-height: 0 !important; margin: 0 !important; padding: 0 !important; border: 0 !important; \
z-index: 2147483647 !important; object-fit: contain !important; background: #000 !important; \
transform: none !important; border-radius: 0 !important; } \
[data-ion-pip]::-webkit-media-controls { display: none !important; } \
[data-ion-pip-up] { transform: none !important; filter: none !important; contain: none !important; \
perspective: none !important; will-change: auto !important; backdrop-filter: none !important; }";

/// Marks `el` as the element to fill the view, and lifts the ancestors that
/// would otherwise trap a fixed-position child.
fn mark_function() -> String {
    format!(
        r#"const ionPipMark = el => {{
  document.documentElement.setAttribute("data-ion-pip-root", "");
  el.setAttribute("data-ion-pip", "");
  for (let up = el.parentElement; up; up = up.parentElement) up.setAttribute("data-ion-pip-up", "");
  let style = document.getElementById("ion-pip-style");
  if (!style) {{
    style = document.createElement("style");
    style.id = "ion-pip-style";
    (document.head || document.documentElement).appendChild(style);
  }}
  style.textContent = {style};
}};"#,
        style = js_string(STYLE)
    )
}

/// Run in the frame [`PICK_SCRIPT`] chose: makes its best video fill the
/// view. Returns `"{width}x{height}"` of the video, or `""`.
pub fn enter_script() -> String {
    format!(
        r#"(() => {{
  {mark}
  let best = null, bestScore = -1;
  for (const video of document.querySelectorAll("video")) {{
    const box = video.getBoundingClientRect();
    const area = box.width * box.height;
    if (video.readyState < 1 || area < 1) continue;
    const asked = video.hasAttribute("data-ion-pip-request") ? 1000 : 1;
    const score = area * asked * (!video.paused && !video.ended ? 4 : 1) * (video.muted || video.volume === 0 ? 1 : 2);
    if (score > bestScore) {{ best = video; bestScore = score; }}
  }}
  if (!best) return "";
  ionPipMark(best);
  return best.videoWidth + "x" + best.videoHeight;
}})()"#,
        mark = mark_function()
    )
}

/// Run in a frame above the video's frame: makes the iframe showing
/// `child_url` fill the view.
pub fn frame_script(child_url: &str) -> String {
    format!(
        r#"(() => {{
  {mark}
  const target = {url};
  for (const frame of document.querySelectorAll("iframe, frame")) {{
    if (frame.src === target || (frame.src && target.startsWith(frame.src))) {{ ionPipMark(frame); return true; }}
  }}
  return false;
}})()"#,
        mark = mark_function(),
        url = js_string(child_url)
    )
}

/// Run in every frame on leaving picture-in-picture: undoes the marks.
pub const EXIT_SCRIPT: &str = r#"(() => {
  if (document.querySelector("[data-ion-pip-request]")) document.dispatchEvent(new Event("ion-pip-left"));
  for (const attr of ["data-ion-pip", "data-ion-pip-up", "data-ion-pip-root", "data-ion-pip-request"])
    for (const el of document.querySelectorAll("[" + attr + "]")) el.removeAttribute(attr);
  const style = document.getElementById("ion-pip-style");
  if (style) style.remove();
})()"#;

/// A picture-in-picture control on the marked video.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Control {
    /// Only report the state.
    State,
    TogglePlay,
    Pause,
    /// Seek by this many seconds.
    Seek(f64),
    ToggleMute,
}

/// Run in the video's frame: applies `control` to the marked video and
/// returns its state as JSON `{ paused, muted, time, duration }`, or `""`
/// when the frame has no marked video.
pub fn control_script(control: Control) -> String {
    let action = match control {
        Control::State => String::new(),
        Control::TogglePlay => "if (v.paused) v.play(); else v.pause();".to_owned(),
        Control::Pause => "v.pause();".to_owned(),
        Control::Seek(seconds) if seconds.is_finite() => format!(
            "v.currentTime = Math.max(0, Math.min(v.duration || 0, v.currentTime + ({seconds})));"
        ),
        Control::Seek(_) => String::new(),
        Control::ToggleMute => "v.muted = !v.muted;".to_owned(),
    };
    format!(
        r#"(() => {{
  const v = document.querySelector("video[data-ion-pip]");
  if (!v) return "";
  {action}
  return JSON.stringify({{ paused: v.paused, muted: v.muted, time: v.currentTime || 0,
    duration: isFinite(v.duration) ? v.duration : 0 }});
}})()"#
    )
}

/// `value` as a JavaScript string literal.
pub fn js_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '<' => out.push_str("\\u003c"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The window size for a video of `video_w`×`video_h`: `width` wide (the
/// height following the video's shape, within 16:9 portrait and 21:9
/// landscape), 16:9 when the size is unknown.
pub fn window_size(video_w: u32, video_h: u32, width: u32) -> (u32, u32) {
    let ratio = if video_w > 0 && video_h > 0 {
        (f64::from(video_w) / f64::from(video_h)).clamp(9.0 / 16.0, 21.0 / 9.0)
    } else {
        16.0 / 9.0
    };
    let height = (f64::from(width) / ratio).round() as u32;
    (width, height.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_quoted_safely() {
        assert_eq!(js_string(r#"a"b\c"#), r#""a\"b\\c""#);
        assert_eq!(js_string("</script>\n"), r#""\u003c/script>\n""#);
        let script = frame_script("https://x.test/embed?\"q\"");
        assert!(script.contains(r#""https://x.test/embed?\"q\"""#));
    }

    #[test]
    fn style_is_one_valid_string() {
        let mark = mark_function();
        assert!(mark.contains("style.textContent = \"html[data-ion-pip-root]"));
        assert!(!STYLE.contains('"'));
    }

    #[test]
    fn controls_turn_into_actions() {
        assert!(control_script(Control::TogglePlay).contains("v.play()"));
        assert!(control_script(Control::Seek(-10.0)).contains("+ (-10)"));
        assert!(!control_script(Control::Seek(f64::NAN)).contains("currentTime ="));
        assert!(!control_script(Control::State).contains("v.pause()"));
    }

    #[test]
    fn page_script_logs_ions_messages() {
        let script = page_script();
        assert!(script.contains(&format!("log(\"{ENTER_MESSAGE}\")")));
        assert!(script.contains(&format!("log(\"{EXIT_MESSAGE}\")")));
        assert!(!script.contains("\"ENTER\""));
    }

    #[test]
    fn window_follows_the_video_shape() {
        assert_eq!(window_size(1920, 1080, 480), (480, 270));
        assert_eq!(window_size(0, 0, 480), (480, 270));
        assert_eq!(window_size(1080, 1920, 270), (270, 480));
        assert_eq!(window_size(100, 1, 480), (480, 206));
    }
}
