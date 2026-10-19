//! Touchpad pinch to zoom the page.
//!
//! Pinching changes the page zoom smoothly, from the zoom the page had when
//! the fingers touched down. Letting go settles on a zoom step when the
//! result is close to one, so a pinch back to roughly 100% lands on 100%.

/// Smallest and largest page zoom, matching the zoom steps.
pub const MIN: f64 = 0.25;
pub const MAX: f64 = 5.0;

/// How close (as a fraction) a released zoom must be to a step to snap to it.
const SNAP: f64 = 0.04;

/// One pinch gesture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pinch {
    start: f64,
    scale: f64,
}

impl Pinch {
    /// Fingers touched down on a page zoomed to `factor`.
    pub fn begin(factor: f64) -> Self {
        Self {
            start: factor.clamp(MIN, MAX),
            scale: 1.0,
        }
    }

    /// The fingers moved: `delta` is the platform's scale change since the
    /// last event (0.05 is 5% bigger). Returns the zoom to show.
    pub fn update(&mut self, delta: f64) -> f64 {
        if delta.is_finite() {
            self.scale = (self.scale * (1.0 + delta)).clamp(MIN / self.start, MAX / self.start);
        }
        self.factor()
    }

    /// The zoom to show now.
    pub fn factor(&self) -> f64 {
        (self.start * self.scale).clamp(MIN, MAX)
    }

    /// Fingers lifted: the zoom to keep, snapped to one of `steps` if close.
    pub fn end(&self, steps: &[f64]) -> f64 {
        let factor = self.factor();
        steps
            .iter()
            .copied()
            .find(|step| ((factor - step) / step).abs() <= SNAP)
            .unwrap_or(factor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEPS: &[f64] = &[0.5, 1.0, 1.5, 2.0];

    #[test]
    fn pinching_scales_from_the_starting_zoom() {
        let mut pinch = Pinch::begin(1.5);
        for _ in 0..10 {
            pinch.update(0.1);
        }
        assert!((pinch.factor() - 1.5 * 1.1f64.powi(10)).abs() < 1e-9);
    }

    #[test]
    fn zoom_stays_in_bounds() {
        let mut pinch = Pinch::begin(1.0);
        for _ in 0..100 {
            pinch.update(0.5);
        }
        assert_eq!(pinch.factor(), MAX);
        // Coming back works straight away, without undoing the overshoot.
        pinch.update(-0.5);
        assert!(pinch.factor() < MAX);
        for _ in 0..100 {
            pinch.update(-0.5);
        }
        assert_eq!(pinch.factor(), MIN);
        pinch.update(f64::NAN);
        assert_eq!(pinch.factor(), MIN);
    }

    #[test]
    fn letting_go_snaps_to_a_nearby_step() {
        let mut pinch = Pinch::begin(1.0);
        pinch.update(0.03);
        assert_eq!(pinch.end(STEPS), 1.0);
        pinch.update(0.2);
        let free = pinch.factor();
        assert_eq!(pinch.end(STEPS), free);
    }
}
