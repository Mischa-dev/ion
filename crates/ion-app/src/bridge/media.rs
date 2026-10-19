//! `Media` QML singleton: autoplay policy, picture-in-picture scripts and
//! media sessions, from `ion_media`, plus the system's media controls (MPRIS
//! on Linux, C++ in `cpp/mpris.cpp`). The picture-in-picture window is
//! `PictureInPicture.qml`; the now-playing controls are `NowPlaying.qml`.

// `publish` takes one argument per track field; QML can't pass a struct.
#![allow(clippy::too_many_arguments)]

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/mpris.h");

        #[cxx_name = "mprisUpdate"]
        fn mpris_update(
            active: bool,
            title: &QString,
            artist: &QString,
            album: &QString,
            art_url: &QString,
            url: &QString,
            length_us: i64,
            position_us: i64,
            playing: bool,
            can_previous: bool,
            can_next: bool,
        );
    }

    #[namespace = "ion"]
    extern "Rust" {
        /// Called by the MPRIS object when people use the system's controls.
        #[cxx_name = "mprisAction"]
        fn mpris_action(action: &QString, value: f64);
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(
            QString,
            block_autoplay_script,
            cxx_name = "blockAutoplayScript",
            READ,
            CONSTANT
        )]
        #[qproperty(
            QString,
            unblock_autoplay_script,
            cxx_name = "unblockAutoplayScript",
            READ,
            CONSTANT
        )]
        #[qproperty(QString, pip_page_script, cxx_name = "pipPageScript", READ, CONSTANT)]
        #[qproperty(QString, pip_pick_script, cxx_name = "pipPickScript", READ, CONSTANT)]
        #[qproperty(QString, pip_enter_script, cxx_name = "pipEnterScript", READ, CONSTANT)]
        #[qproperty(QString, pip_exit_script, cxx_name = "pipExitScript", READ, CONSTANT)]
        #[qproperty(
            QString,
            pip_enter_message,
            cxx_name = "pipEnterMessage",
            READ,
            CONSTANT
        )]
        #[qproperty(QString, pip_exit_message, cxx_name = "pipExitMessage", READ, CONSTANT)]
        #[qproperty(QString, session_script, cxx_name = "sessionScript", READ, CONSTANT)]
        #[namespace = "ion"]
        type Media = super::MediaRust;

        /// What pages at `url` may play on their own: `"allow"`, `"muted"`
        /// or `"block"` (`media.autoplay`, after `[sites]` overrides).
        #[qinvokable]
        fn autoplay(self: &Media, url: &QString) -> QString;

        /// Script for a frame above the video's: makes the iframe showing
        /// `child_url` fill the view.
        #[qinvokable]
        #[cxx_name = "pipFrameScript"]
        fn pip_frame_script(self: &Media, child_url: &QString) -> QString;

        /// Script for the video's frame: `action` ("state", "toggle",
        /// "pause", "seek", "mute") on the picture-in-picture video, `seconds`
        /// for "seek". It returns `{ paused, muted, time, duration }` JSON.
        #[qinvokable]
        #[cxx_name = "pipControlScript"]
        fn pip_control_script(self: &Media, action: &QString, seconds: f64) -> QString;

        /// Height of a `width`-wide picture-in-picture window for a video of
        /// `video_width`×`video_height` (0 when unknown).
        #[qinvokable]
        #[cxx_name = "pipHeight"]
        fn pip_height(self: &Media, video_width: i32, video_height: i32, width: i32) -> i32;

        /// A console message from the session script as `{ frame, state }`,
        /// `state` being `{ playing, title, artist, album, artwork,
        /// duration, position, hasVideo, canPrevious, canNext }` or null
        /// once that frame has nothing playable. Null for other messages.
        #[qinvokable]
        #[cxx_name = "sessionReport"]
        fn session_report(self: &Media, message: &QString) -> QVariant;

        /// Script (Ion's world, each frame) asking the page to `action`:
        /// "play", "pause", "playpause", "stop", "next", "previous",
        /// "seekby" or "seekto" (both with `value` in seconds). "" for
        /// anything else.
        #[qinvokable]
        #[cxx_name = "actionScript"]
        fn action_script(self: &Media, action: &QString, value: f64) -> QString;

        /// Show this track in the system's media controls, or with `active`
        /// false take Ion out of them. Seconds for `duration` and `position`.
        #[qinvokable]
        fn publish(
            self: &Media,
            active: bool,
            title: &QString,
            artist: &QString,
            album: &QString,
            artwork: &QString,
            url: &QString,
            duration: f64,
            position: f64,
            playing: bool,
            can_previous: bool,
            can_next: bool,
        );

        /// The system's media controls asked for `action` (as for
        /// `actionScript`, plus "raise").
        #[qsignal]
        #[cxx_name = "systemAction"]
        fn system_action(self: Pin<&mut Media>, action: &QString, value: f64);
    }

    impl cxx_qt::Threading for Media {}
    impl cxx_qt::Initialize for Media {}
}

use std::pin::Pin;
use std::sync::Mutex;

use cxx_qt::{CxxQtThread, Threading};
use cxx_qt_lib::{QMap, QMapPair_QString_QVariant, QString, QVariant};
use ion_media::pip::{self, Control};
use ion_media::session::{self, Action};

/// The singleton's thread, for actions arriving from the system.
static THREAD: Mutex<Option<CxxQtThread<qobject::Media>>> = Mutex::new(None);

fn mpris_action(action: &QString, value: f64) {
    let action = action.to_string();
    if let Some(thread) = THREAD.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = thread.queue(move |media| {
            media.system_action(&QString::from(action.as_str()), value);
        });
    }
}

impl cxx_qt::Initialize for qobject::Media {
    fn initialize(self: Pin<&mut Self>) {
        *THREAD.lock().unwrap_or_else(|e| e.into_inner()) = Some(self.qt_thread());
    }
}

/// Seconds to whole microseconds, as MPRIS counts.
fn micros(seconds: f64) -> i64 {
    if seconds.is_finite() && seconds > 0.0 {
        (seconds * 1e6).round().min(i64::MAX as f64) as i64
    } else {
        0
    }
}

pub struct MediaRust {
    block_autoplay_script: QString,
    unblock_autoplay_script: QString,
    pip_page_script: QString,
    pip_pick_script: QString,
    pip_enter_script: QString,
    pip_exit_script: QString,
    pip_enter_message: QString,
    pip_exit_message: QString,
    session_script: QString,
}

impl Default for MediaRust {
    fn default() -> Self {
        Self {
            block_autoplay_script: QString::from(ion_media::autoplay::BLOCK_SCRIPT),
            unblock_autoplay_script: QString::from(ion_media::autoplay::UNBLOCK_SCRIPT),
            pip_page_script: QString::from(pip::page_script().as_str()),
            pip_pick_script: QString::from(pip::PICK_SCRIPT),
            pip_enter_script: QString::from(pip::enter_script().as_str()),
            pip_exit_script: QString::from(pip::EXIT_SCRIPT),
            pip_enter_message: QString::from(pip::ENTER_MESSAGE),
            pip_exit_message: QString::from(pip::EXIT_MESSAGE),
            session_script: QString::from(session::page_script().as_str()),
        }
    }
}

impl qobject::Media {
    fn autoplay(&self, url: &QString) -> QString {
        let config = ion_config::global().config();
        let policy =
            ion_media::autoplay_for(config.media.autoplay, &config.sites, &url.to_string());
        QString::from(policy.as_str())
    }

    fn pip_frame_script(&self, child_url: &QString) -> QString {
        QString::from(pip::frame_script(&child_url.to_string()).as_str())
    }

    fn pip_control_script(&self, action: &QString, seconds: f64) -> QString {
        let control = match action.to_string().as_str() {
            "toggle" => Control::TogglePlay,
            "pause" => Control::Pause,
            "seek" => Control::Seek(seconds),
            "mute" => Control::ToggleMute,
            _ => Control::State,
        };
        QString::from(pip::control_script(control).as_str())
    }

    fn pip_height(&self, video_width: i32, video_height: i32, width: i32) -> i32 {
        let size = |n: i32| u32::try_from(n).unwrap_or(0);
        let (_, height) = pip::window_size(size(video_width), size(video_height), size(width));
        i32::try_from(height).unwrap_or(i32::MAX)
    }

    fn session_report(&self, message: &QString) -> QVariant {
        let message = message.to_string();
        let Some((frame, json)) = session::split(&message) else {
            return QVariant::default();
        };
        let state = session::parse(json).map_or_else(QVariant::default, |s| state_map(&s));
        let mut map = QMap::<QMapPair_QString_QVariant>::default();
        map.insert(
            QString::from("frame"),
            QVariant::from(&QString::from(frame)),
        );
        map.insert(QString::from("state"), state);
        QVariant::from(&map)
    }
}

/// A page's media state as the map QML reads.
fn state_map(state: &session::State) -> QVariant {
    let mut map = QMap::<QMapPair_QString_QVariant>::default();
    let mut text = |key: &str, value: &str| {
        map.insert(QString::from(key), QVariant::from(&QString::from(value)));
    };
    text("title", &state.title);
    text("artist", &state.artist);
    text("album", &state.album);
    text("artwork", &state.artwork);
    for (key, value) in [
        ("playing", state.playing),
        ("hasVideo", state.has_video),
        ("canPrevious", state.can_previous),
        ("canNext", state.can_next),
    ] {
        map.insert(QString::from(key), QVariant::from(&value));
    }
    for (key, value) in [("duration", state.duration), ("position", state.position)] {
        map.insert(QString::from(key), QVariant::from(&value));
    }
    QVariant::from(&map)
}

impl qobject::Media {
    fn action_script(&self, action: &QString, value: f64) -> QString {
        match Action::from_name(&action.to_string(), value) {
            Some(action) => QString::from(session::action_script(action).as_str()),
            None => QString::default(),
        }
    }

    fn publish(
        &self,
        active: bool,
        title: &QString,
        artist: &QString,
        album: &QString,
        artwork: &QString,
        url: &QString,
        duration: f64,
        position: f64,
        playing: bool,
        can_previous: bool,
        can_next: bool,
    ) {
        qobject::mpris_update(
            active,
            title,
            artist,
            album,
            artwork,
            url,
            micros(duration),
            micros(position),
            playing,
            can_previous,
            can_next,
        );
    }
}
