//! `Gestures` QML element: two-finger swipes for back and forward and pinch
//! to zoom, from `ion_input::swipe` and `ion_input::pinch`, following
//! `[gestures]` in the config.
//!
//! `attach(window)` installs an event filter (`cpp/gestures.cpp`) that only
//! watches touchpad scrolling and pinching and reports it to `scroll`,
//! `pause` and `pinch`. QML keeps `area`, `canGoBack`, `canGoForward` and
//! `zoomFactor` in step with the current page, draws the arrow from
//! `swipeDirection` / `swipeProgress`, and acts on `navigate` and `zoom`.
//! On `swipeStarted` it checks what is under the fingers and calls `block`
//! for a direction something there can still scroll.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qrectf.h");
        type QRectF = cxx_qt_lib::QRectF;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QRectF, area)]
        #[qproperty(bool, can_go_back, cxx_name = "canGoBack")]
        #[qproperty(bool, can_go_forward, cxx_name = "canGoForward")]
        #[qproperty(f64, zoom_factor, cxx_name = "zoomFactor")]
        #[qproperty(QString, swipe_direction, cxx_name = "swipeDirection", READ, NOTIFY)]
        #[qproperty(f64, swipe_progress, cxx_name = "swipeProgress", READ, NOTIFY)]
        #[namespace = "ion"]
        type Gestures = super::GesturesRust;

        /// Start watching `window` (the ApplicationWindow).
        #[qinvokable]
        unsafe fn attach(self: Pin<&mut Gestures>, window: *mut QObject) -> bool;

        /// A touchpad scroll event, from the event filter. Never swallows it.
        #[qinvokable]
        fn scroll(self: Pin<&mut Gestures>, dx: f64, dy: f64, phase: i32, x: f64, y: f64) -> bool;

        /// No phased scroll events for a moment, from the event filter.
        #[qinvokable]
        fn pause(self: Pin<&mut Gestures>);

        /// A pinch step (0 begin, 1 zoom by `delta`, 2 end), from the event
        /// filter. Returns whether to swallow it: zoom steps Ion handles,
        /// never begin or end.
        #[qinvokable]
        fn pinch(self: Pin<&mut Gestures>, stage: i32, delta: f64, x: f64, y: f64) -> bool;

        /// Something under the fingers can still scroll back (left) or
        /// forward (right): gesture `gesture` won't navigate that way.
        #[qinvokable]
        fn block(self: Pin<&mut Gestures>, gesture: i32, back: bool, forward: bool);

        /// A touchpad gesture began at (`x`, `y`) in the window. Answer
        /// with `block` (all false if nothing there scrolls sideways): the
        /// gesture doesn't navigate until it does.
        #[qsignal]
        #[cxx_name = "swipeStarted"]
        fn swipe_started(self: Pin<&mut Gestures>, gesture: i32, x: f64, y: f64);

        /// A swipe went far enough: go "back" or "forward".
        #[qsignal]
        fn navigate(self: Pin<&mut Gestures>, direction: QString);

        /// Show the page at `factor`; `done` when the fingers lifted (save it).
        #[qsignal]
        fn zoom(self: Pin<&mut Gestures>, factor: f64, done: bool);
    }

    #[namespace = "ion"]
    unsafe extern "C++" {
        include!("ion-app/cpp/gestures.h");

        #[cxx_name = "installGestureFilter"]
        unsafe fn install_gesture_filter(window: *mut QObject, target: *mut QObject) -> bool;
    }
}

use std::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt::casting::Upcast;
use cxx_qt_lib::{QRectF, QString};
use ion_config::PinchAction;
use ion_input::pinch::Pinch;
use ion_input::swipe::{Direction, Outcome, Phase, Scroll, Swipe};

#[derive(Default)]
pub struct GesturesRust {
    area: QRectF,
    can_go_back: bool,
    can_go_forward: bool,
    zoom_factor: f64,
    swipe_direction: QString,
    swipe_progress: f64,
    swipe: Swipe,
    pinch: Option<Pinch>,
    attached: bool,
}

fn phase(qt: i32) -> Phase {
    // Qt::ScrollPhase.
    match qt {
        1 => Phase::Begin,
        2 => Phase::Update,
        3 => Phase::End,
        4 => Phase::Momentum,
        _ => Phase::Unknown,
    }
}

fn contains(area: &QRectF, x: f64, y: f64) -> bool {
    x >= area.x() && y >= area.y() && x < area.x() + area.width() && y < area.y() + area.height()
}

impl qobject::Gestures {
    unsafe fn attach(mut self: Pin<&mut Self>, window: *mut qobject::QObject) -> bool {
        if self.attached {
            return true;
        }
        let target: *mut qobject::QObject = {
            let base: Pin<&mut qobject::QObject> = self.as_mut().upcast_pin();
            // SAFETY: only the address is taken; the object stays pinned.
            unsafe { base.get_unchecked_mut() }
        };
        // SAFETY: `window` is the live ApplicationWindow from QML (or null,
        // which the shim rejects); this object lives as long as the window.
        let ok = unsafe { qobject::install_gesture_filter(window, target) };
        self.as_mut().rust_mut().attached = ok;
        ok
    }

    fn show_swipe(mut self: Pin<&mut Self>) {
        let (direction, progress) = match self.swipe.progress() {
            Some((Direction::Back, p)) => ("back", p),
            Some((Direction::Forward, p)) => ("forward", p),
            None => ("", 0.0),
        };
        let direction = QString::from(direction);
        if self.swipe_direction != direction {
            self.as_mut().rust_mut().swipe_direction = direction;
            self.as_mut().swipe_direction_changed();
        }
        if self.swipe_progress != progress {
            self.as_mut().rust_mut().swipe_progress = progress;
            self.as_mut().swipe_progress_changed();
        }
    }

    fn outcome(mut self: Pin<&mut Self>, outcome: Outcome) {
        match outcome {
            Outcome::None => {}
            Outcome::Moved | Outcome::Cancelled => self.show_swipe(),
            Outcome::Navigate(direction) => {
                self.as_mut().show_swipe();
                let direction = match direction {
                    Direction::Back => "back",
                    Direction::Forward => "forward",
                };
                self.navigate(QString::from(direction));
            }
        }
    }

    fn scroll(mut self: Pin<&mut Self>, dx: f64, dy: f64, qt_phase: i32, x: f64, y: f64) -> bool {
        // A gesture that starts outside the page (the tab list, a popup)
        // can't swipe: the recognizer reads the edges as the gesture starts.
        let enabled = ion_config::global().config().gestures.swipe && contains(&self.area, x, y);
        let event = Scroll {
            dx,
            dy,
            phase: phase(qt_phase),
            can_go_back: enabled && self.can_go_back,
            can_go_forward: enabled && self.can_go_forward,
        };
        let before = self.swipe.gesture();
        let outcome = self.as_mut().rust_mut().swipe.scroll(event);
        let gesture = self.swipe.gesture();
        if gesture != before && (event.can_go_back || event.can_go_forward) {
            // Wraps after 2^31 gestures, which only skips one check.
            self.as_mut().rust_mut().swipe.check();
            self.as_mut().swipe_started(gesture as i32, x, y);
        }
        self.outcome(outcome);
        false
    }

    fn block(mut self: Pin<&mut Self>, gesture: i32, back: bool, forward: bool) {
        let current = self.swipe.gesture();
        if current as i32 != gesture {
            return;
        }
        let outcome = self.as_mut().rust_mut().swipe.block(current, back, forward);
        self.outcome(outcome);
    }

    fn pause(mut self: Pin<&mut Self>) {
        let outcome = self.as_mut().rust_mut().swipe.pause();
        self.outcome(outcome);
    }

    fn pinch(mut self: Pin<&mut Self>, stage: i32, delta: f64, x: f64, y: f64) -> bool {
        let action = ion_config::global().config().gestures.pinch;
        match action {
            PinchAction::Magnify => return false,
            // Only the zoom steps: begin and end bracket other native
            // gestures too.
            PinchAction::Off => return stage == 1,
            PinchAction::Zoom => {}
        }
        match stage {
            0 => {
                self.as_mut().rust_mut().pinch = None;
                false
            }
            1 => {
                if self.pinch.is_none() {
                    if !contains(&self.area, x, y) {
                        return false;
                    }
                    let start = self.zoom_factor;
                    self.as_mut().rust_mut().pinch = Some(Pinch::begin(start));
                }
                let factor = match self.as_mut().rust_mut().pinch.as_mut() {
                    Some(pinch) => pinch.update(delta),
                    None => return false,
                };
                self.zoom(factor, false);
                true
            }
            // Begin and end always reach the page, so its own gesture
            // handling sees a whole (empty) gesture; only zoom steps are
            // taken.
            2 => {
                if let Some(pinch) = self.as_mut().rust_mut().pinch.take() {
                    self.zoom(pinch.end(&ion_basics::zoom::STEPS), true);
                }
                false
            }
            _ => false,
        }
    }
}
