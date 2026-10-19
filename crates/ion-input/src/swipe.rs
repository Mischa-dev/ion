//! Two-finger touchpad swipes for back and forward.
//!
//! Touchpads report a swipe as a run of scroll events. Like Chrome and
//! Safari, Ion treats horizontal scrolling past the page's edge as a swipe:
//! scrolling left from a page already at its left edge goes back, scrolling
//! right from its right edge goes forward. The page still gets every event.
//! A carousel or editor under the fingers that can still scroll that way
//! scrolls instead: the app checks once a gesture starts and calls
//! [`Swipe::block`].
//!
//! The recognizer only reads events; `ion-app` draws the arrow from
//! [`Swipe::progress`] and navigates on [`Outcome::Navigate`].

/// Where a scroll event sits in a gesture, as the platform reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Fingers touched down.
    Begin,
    Update,
    /// Fingers lifted.
    End,
    /// Inertia after the fingers lifted (macOS). Never starts a swipe.
    Momentum,
    /// The platform doesn't say (some X11 drivers); a pause ends the gesture.
    Unknown,
}

/// The direction of a swipe that navigates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Back,
    Forward,
}

/// One scroll event from a touchpad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scroll {
    /// Pixel deltas as Qt reports them: positive `dx` scrolls toward the
    /// page's left edge.
    pub dx: f64,
    pub dy: f64,
    pub phase: Phase,
    /// The page can go back and is scrolled to its left edge.
    pub can_go_back: bool,
    /// The page can go forward and is scrolled to its right edge.
    pub can_go_forward: bool,
}

/// What a scroll event changed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// Nothing to show.
    None,
    /// A swipe is under way; redraw with [`Swipe::progress`].
    Moved,
    /// A swipe stopped short; hide the arrow.
    Cancelled,
    /// A swipe went far enough: navigate.
    Navigate(Direction),
}

/// Tuning, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// Horizontal travel a swipe needs to navigate.
    pub distance: f64,
    /// Travel before the recognizer decides between horizontal and vertical.
    pub slop: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            distance: 160.0,
            slop: 10.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Idle,
    /// Fingers down, axis not decided yet.
    Deciding {
        dx: f64,
        dy: f64,
    },
    /// A horizontal swipe toward `direction`; `dx` is signed travel.
    Swiping {
        direction: Direction,
        dx: f64,
    },
    /// The rest of this gesture is ordinary scrolling.
    Ignoring,
    /// Let go far enough to navigate, but the check of what was under the
    /// fingers hasn't answered yet.
    Waiting {
        direction: Direction,
    },
}

/// Turns scroll events into back/forward swipes.
#[derive(Debug, Clone)]
pub struct Swipe {
    settings: Settings,
    state: State,
    can_go_back: bool,
    can_go_forward: bool,
    gesture: u64,
    /// This gesture waits for [`Swipe::block`] before it navigates.
    checking: bool,
}

impl Default for Swipe {
    fn default() -> Self {
        Self::new(Settings::default())
    }
}

impl Swipe {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            state: State::Idle,
            can_go_back: false,
            can_go_forward: false,
            gesture: 0,
            checking: false,
        }
    }

    /// Hold back navigating in the current gesture until [`Swipe::block`]
    /// answers for it.
    pub fn check(&mut self) {
        self.checking = true;
    }

    /// Counts gestures: changes each time one starts, so a check started
    /// for one gesture can tell whether it still applies.
    pub fn gesture(&self) -> u64 {
        self.gesture
    }

    /// Something under the fingers can still scroll back (left) or forward
    /// (right), so gesture `gesture` must not navigate that way. Returns
    /// [`Outcome::Cancelled`] when that stops a swipe already showing.
    pub fn block(&mut self, gesture: u64, back: bool, forward: bool) -> Outcome {
        if gesture != self.gesture {
            return Outcome::None;
        }
        self.can_go_back &= !back;
        self.can_go_forward &= !forward;
        self.checking = false;
        match self.state {
            State::Waiting { direction } => {
                self.state = State::Idle;
                let blocked = match direction {
                    Direction::Back => back,
                    Direction::Forward => forward,
                };
                if blocked {
                    Outcome::Cancelled
                } else {
                    Outcome::Navigate(direction)
                }
            }
            State::Swiping { direction, .. }
                if (direction == Direction::Back && back)
                    || (direction == Direction::Forward && forward) =>
            {
                self.state = State::Ignoring;
                Outcome::Cancelled
            }
            _ => Outcome::None,
        }
    }

    /// The swipe under way and how far along it is, from 0 to 1 (1 means
    /// letting go navigates).
    pub fn progress(&self) -> Option<(Direction, f64)> {
        match self.state {
            State::Swiping { direction, dx } => {
                let signed = match direction {
                    Direction::Back => dx,
                    Direction::Forward => -dx,
                };
                Some((direction, (signed / self.settings.distance).clamp(0.0, 1.0)))
            }
            State::Waiting { direction } => Some((direction, 1.0)),
            _ => None,
        }
    }

    /// Feed one scroll event.
    pub fn scroll(&mut self, event: Scroll) -> Outcome {
        match event.phase {
            Phase::Momentum => return Outcome::None,
            Phase::End => return self.finish(),
            Phase::Begin => {
                let was_swiping =
                    matches!(self.state, State::Swiping { .. } | State::Waiting { .. });
                self.start(event);
                if was_swiping {
                    return Outcome::Cancelled;
                }
            }
            Phase::Update | Phase::Unknown => {
                if let State::Waiting { .. } = self.state {
                    // The check never answered; a new gesture takes over.
                    self.start(event);
                    self.travel(event.dx, event.dy);
                    return Outcome::Cancelled;
                }
                if self.state == State::Idle {
                    self.start(event);
                }
            }
        }
        self.travel(event.dx, event.dy)
    }

    /// No events for a while: the end of a gesture whose platform reports
    /// no phases.
    pub fn pause(&mut self) -> Outcome {
        self.finish()
    }

    fn start(&mut self, event: Scroll) {
        self.state = State::Deciding { dx: 0.0, dy: 0.0 };
        self.gesture += 1;
        self.checking = false;
        // Edges as the gesture starts; scrolling during it moves the page.
        self.can_go_back = event.can_go_back;
        self.can_go_forward = event.can_go_forward;
    }

    fn travel(&mut self, ddx: f64, ddy: f64) -> Outcome {
        match self.state {
            State::Idle | State::Ignoring | State::Waiting { .. } => Outcome::None,
            State::Deciding { dx, dy } => {
                let (dx, dy) = (dx + ddx, dy + ddy);
                if dx.hypot(dy) < self.settings.slop {
                    self.state = State::Deciding { dx, dy };
                    return Outcome::None;
                }
                // Clearly sideways, toward an edge the page can leave by.
                let sideways = dx.abs() > dy.abs() * 2.0;
                let direction = if dx > 0.0 {
                    self.can_go_back.then_some(Direction::Back)
                } else {
                    self.can_go_forward.then_some(Direction::Forward)
                };
                match direction.filter(|_| sideways) {
                    Some(direction) => {
                        self.state = State::Swiping { direction, dx };
                        Outcome::Moved
                    }
                    None => {
                        self.state = State::Ignoring;
                        Outcome::None
                    }
                }
            }
            State::Swiping { direction, dx } => {
                // Moving back past the start just shows no arrow.
                self.state = State::Swiping {
                    direction,
                    dx: dx + ddx,
                };
                Outcome::Moved
            }
        }
    }

    fn finish(&mut self) -> Outcome {
        if matches!(self.state, State::Waiting { .. }) {
            return Outcome::None;
        }
        let progress = self.progress();
        self.state = State::Idle;
        match progress {
            Some((direction, amount)) if amount >= 1.0 && self.checking => {
                self.state = State::Waiting { direction };
                Outcome::Moved
            }
            Some((direction, amount)) if amount >= 1.0 => Outcome::Navigate(direction),
            Some(_) => Outcome::Cancelled,
            None => Outcome::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scroll(dx: f64, dy: f64, phase: Phase) -> Scroll {
        Scroll {
            dx,
            dy,
            phase,
            can_go_back: true,
            can_go_forward: true,
        }
    }

    fn run(swipe: &mut Swipe, steps: usize, dx: f64, dy: f64) -> Outcome {
        let mut last = swipe.scroll(scroll(0.0, 0.0, Phase::Begin));
        for _ in 0..steps {
            last = swipe.scroll(scroll(dx, dy, Phase::Update));
        }
        last
    }

    #[test]
    fn a_long_swipe_left_goes_back() {
        let mut swipe = Swipe::default();
        assert_eq!(run(&mut swipe, 20, 10.0, 0.5), Outcome::Moved);
        assert_eq!(swipe.progress(), Some((Direction::Back, 1.0)));
        assert_eq!(
            swipe.scroll(scroll(0.0, 0.0, Phase::End)),
            Outcome::Navigate(Direction::Back)
        );
        assert_eq!(swipe.progress(), None);
    }

    #[test]
    fn a_long_swipe_right_goes_forward() {
        let mut swipe = Swipe::default();
        run(&mut swipe, 20, -10.0, 0.0);
        assert_eq!(swipe.pause(), Outcome::Navigate(Direction::Forward));
    }

    #[test]
    fn a_short_swipe_cancels() {
        let mut swipe = Swipe::default();
        run(&mut swipe, 5, 10.0, 0.0);
        let (_, amount) = swipe.progress().unwrap();
        assert!(amount > 0.2 && amount < 0.5, "{amount}");
        assert_eq!(
            swipe.scroll(scroll(0.0, 0.0, Phase::End)),
            Outcome::Cancelled
        );
    }

    #[test]
    fn swiping_back_to_the_start_cancels() {
        let mut swipe = Swipe::default();
        run(&mut swipe, 20, 10.0, 0.0);
        for _ in 0..20 {
            swipe.scroll(scroll(-10.0, 0.0, Phase::Update));
        }
        assert_eq!(swipe.progress(), Some((Direction::Back, 0.0)));
        assert_eq!(
            swipe.scroll(scroll(0.0, 0.0, Phase::End)),
            Outcome::Cancelled
        );
    }

    #[test]
    fn vertical_scrolling_is_left_alone() {
        let mut swipe = Swipe::default();
        assert_eq!(run(&mut swipe, 20, 3.0, 10.0), Outcome::None);
        assert_eq!(swipe.progress(), None);
        assert_eq!(swipe.scroll(scroll(0.0, 0.0, Phase::End)), Outcome::None);
    }

    #[test]
    fn no_swipe_away_from_the_edge_or_without_history() {
        let mut swipe = Swipe::default();
        swipe.scroll(Scroll {
            can_go_back: false,
            ..scroll(0.0, 0.0, Phase::Begin)
        });
        for _ in 0..20 {
            // The page scrolls meanwhile; the edge seen at the start counts.
            swipe.scroll(scroll(10.0, 0.0, Phase::Update));
        }
        assert_eq!(swipe.progress(), None);
    }

    #[test]
    fn a_scrollable_element_under_the_fingers_blocks_its_direction() {
        let mut swipe = Swipe::default();
        swipe.scroll(scroll(0.0, 0.0, Phase::Begin));
        let gesture = swipe.gesture();
        // The check answers late, mid-swipe: the arrow goes away.
        run_on(&mut swipe, 5, 10.0);
        assert_eq!(swipe.block(gesture, true, false), Outcome::Cancelled);
        run_on(&mut swipe, 20, 10.0);
        assert_eq!(swipe.progress(), None);
        assert_eq!(swipe.scroll(scroll(0.0, 0.0, Phase::End)), Outcome::None);

        // A swipe that ends before the check answers waits for it.
        swipe.scroll(scroll(0.0, 0.0, Phase::Begin));
        swipe.check();
        run_on(&mut swipe, 20, 10.0);
        assert_eq!(swipe.scroll(scroll(0.0, 0.0, Phase::End)), Outcome::Moved);
        assert_eq!(swipe.progress(), Some((Direction::Back, 1.0)));
        assert_eq!(
            swipe.block(swipe.gesture(), false, true),
            Outcome::Navigate(Direction::Back)
        );
        swipe.scroll(scroll(0.0, 0.0, Phase::Begin));
        swipe.check();
        run_on(&mut swipe, 20, 10.0);
        assert_eq!(swipe.pause(), Outcome::Moved);
        assert_eq!(
            swipe.block(swipe.gesture(), true, false),
            Outcome::Cancelled
        );
        assert_eq!(swipe.progress(), None);

        // A check that never answers doesn't stop the next gesture.
        swipe.scroll(scroll(0.0, 0.0, Phase::Begin));
        swipe.check();
        run_on(&mut swipe, 20, 10.0);
        assert_eq!(swipe.pause(), Outcome::Moved);
        assert_eq!(
            swipe.scroll(scroll(10.0, 0.0, Phase::Unknown)),
            Outcome::Cancelled
        );
        swipe.pause();

        // Only the blocked way is off, and only for that gesture.
        swipe.scroll(scroll(0.0, 0.0, Phase::Begin));
        assert_eq!(swipe.block(swipe.gesture(), true, false), Outcome::None);
        run_on(&mut swipe, 20, -10.0);
        assert_eq!(swipe.pause(), Outcome::Navigate(Direction::Forward));
        assert_eq!(swipe.block(gesture, false, true), Outcome::None);
        assert_eq!(run(&mut swipe, 20, 10.0, 0.0), Outcome::Moved);
    }

    fn run_on(swipe: &mut Swipe, steps: usize, dx: f64) {
        for _ in 0..steps {
            swipe.scroll(scroll(dx, 0.0, Phase::Update));
        }
    }

    #[test]
    fn momentum_never_swipes() {
        let mut swipe = Swipe::default();
        for _ in 0..30 {
            assert_eq!(
                swipe.scroll(scroll(10.0, 0.0, Phase::Momentum)),
                Outcome::None
            );
        }
    }

    #[test]
    fn phaseless_events_start_and_pause_ends() {
        let mut swipe = Swipe::default();
        for _ in 0..20 {
            swipe.scroll(scroll(10.0, 0.0, Phase::Unknown));
        }
        assert_eq!(swipe.pause(), Outcome::Navigate(Direction::Back));
        assert_eq!(swipe.pause(), Outcome::None);
    }
}
