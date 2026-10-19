//! Mouse gestures: hold the right button, draw a stroke, let go.
//!
//! A stroke is read as a few straight moves (left, right, up, down), written
//! as letters: "L" is a move left, "DR" is down then right. Short wobbles
//! don't count. Letting go without a recognized stroke is an ordinary right
//! click, so the page's menu still opens.

use std::collections::BTreeMap;

/// Travel, in logical pixels, before a move counts.
const STEP: f64 = 30.0;
/// Longest stroke recognized; anything more is a scribble.
const MAX_MOVES: usize = 4;

/// The built-in strokes and the commands they run.
pub const DEFAULTS: &[(&str, &str)] = &[
    ("L", "back"),
    ("R", "forward"),
    ("D", "new-tab"),
    ("DR", "close-tab"),
    ("DL", "reopen-tab"),
    ("UD", "reload"),
    ("U", "zoom-reset"),
    ("RL", "duplicate-tab"),
];

/// Which command each stroke runs: the defaults with `custom` on top
/// (stroke → command id; an empty id turns a stroke off).
pub fn actions(custom: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut map: BTreeMap<String, String> = DEFAULTS
        .iter()
        .map(|(stroke, id)| (stroke.to_string(), id.to_string()))
        .collect();
    for (stroke, id) in custom {
        let stroke = stroke.trim().to_ascii_uppercase();
        if !drawable(&stroke) {
            continue;
        }
        if id.trim().is_empty() {
            map.remove(&stroke);
        } else {
            map.insert(stroke, id.trim().to_owned());
        }
    }
    map
}

/// Whether `Stroke` can produce `stroke`: up to `MAX_MOVES` letters of LRUD,
/// never the same one twice in a row (one long move is a single letter).
fn drawable(stroke: &str) -> bool {
    let bytes = stroke.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= MAX_MOVES
        && bytes.iter().all(|c| b"LRUD".contains(c))
        && bytes.windows(2).all(|pair| pair[0] != pair[1])
}

/// One stroke being drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    /// Where the current move started.
    anchor: (f64, f64),
    moves: String,
    scribble: bool,
}

impl Stroke {
    pub fn begin(x: f64, y: f64) -> Self {
        Self {
            anchor: (x, y),
            moves: String::new(),
            scribble: false,
        }
    }

    /// The pointer moved to (`x`, `y`).
    pub fn to(&mut self, x: f64, y: f64) {
        let (dx, dy) = (x - self.anchor.0, y - self.anchor.1);
        if dx.hypot(dy) < STEP {
            return;
        }
        let letter = if dx.abs() > dy.abs() {
            if dx < 0.0 { 'L' } else { 'R' }
        } else if dy < 0.0 {
            'U'
        } else {
            'D'
        };
        self.anchor = (x, y);
        if self.moves.ends_with(letter) {
            return;
        }
        if self.moves.len() == MAX_MOVES {
            self.scribble = true;
        } else {
            self.moves.push(letter);
        }
    }

    /// The moves so far ("DR"); empty for a plain click.
    pub fn moves(&self) -> &str {
        if self.scribble { "" } else { &self.moves }
    }

    /// Whether the pointer has moved enough that this isn't a click.
    pub fn moved(&self) -> bool {
        self.scribble || !self.moves.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(points: &[(f64, f64)]) -> Stroke {
        let mut stroke = Stroke::begin(points[0].0, points[0].1);
        for &(x, y) in &points[1..] {
            stroke.to(x, y);
        }
        stroke
    }

    #[test]
    fn reads_straight_moves() {
        assert_eq!(
            draw(&[(100.0, 100.0), (60.0, 105.0), (20.0, 98.0)]).moves(),
            "L"
        );
        assert_eq!(
            draw(&[
                (0.0, 0.0),
                (5.0, 40.0),
                (8.0, 90.0),
                (60.0, 95.0),
                (120.0, 92.0)
            ])
            .moves(),
            "DR"
        );
        assert_eq!(draw(&[(0.0, 0.0), (0.0, -50.0), (0.0, 10.0)]).moves(), "UD");
    }

    #[test]
    fn small_wobbles_are_a_click() {
        let stroke = draw(&[(0.0, 0.0), (5.0, 3.0), (-4.0, 8.0), (10.0, -10.0)]);
        assert_eq!(stroke.moves(), "");
        assert!(!stroke.moved());
    }

    #[test]
    fn scribbles_do_nothing() {
        let mut points = vec![(0.0, 0.0)];
        for i in 0..10 {
            points.push(if i % 2 == 0 { (100.0, 0.0) } else { (0.0, 0.0) });
        }
        let stroke = draw(&points);
        assert_eq!(stroke.moves(), "");
        assert!(stroke.moved());
    }

    #[test]
    fn custom_strokes_override_and_remove_defaults() {
        let custom = BTreeMap::from([
            ("u".to_string(), "new-tab".to_string()),
            ("D".to_string(), String::new()),
            ("X".to_string(), "quit".to_string()),
        ]);
        let map = actions(&custom);
        assert_eq!(map["U"], "new-tab");
        assert!(!map.contains_key("D"));
        assert!(!map.contains_key("X"));
        assert_eq!(map["L"], "back");
    }

    #[test]
    fn strokes_that_cannot_be_drawn_are_ignored() {
        let custom = BTreeMap::from([
            ("LL".to_string(), "new-tab".to_string()),
            ("LRUDL".to_string(), "quit".to_string()),
            ("LRUD".to_string(), "reload".to_string()),
        ]);
        let map = actions(&custom);
        assert!(!map.contains_key("LL"));
        assert!(!map.contains_key("LRUDL"));
        assert_eq!(map["LRUD"], "reload");
    }
}
