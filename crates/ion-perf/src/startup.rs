//! Startup marks.
//!
//! `main` calls [`begin`] first thing; later steps call [`mark`] with a name
//! (`engine`, `qml`, `first-frame`, `first-page`). Each name is kept once, as
//! the time since `begin`. With `ION_PERF=1` every mark and [`event`] is also
//! printed to stderr in a fixed format that `bench/bench.py` parses:
//!
//! ```text
//! ion-perf mark first-frame main_ms=412.3 unix_ms=1760000000123
//! ion-perf event page-loaded main_ms=980.1 unix_ms=1760000000691
//! ```
//!
//! `unix_ms` lets the benchmark add the time before `main` (process start,
//! dynamic linking), which it measures from the moment it launched Ion.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Set to `1` to print marks and events to stderr.
pub const ENV_VAR: &str = "ION_PERF";

static START: OnceLock<Instant> = OnceLock::new();
static MARKS: Mutex<Vec<(String, Duration)>> = Mutex::new(Vec::new());

/// Start the clock. Later calls keep the first start.
pub fn begin() {
    START.get_or_init(Instant::now);
    mark("main");
}

/// Whether marks are printed (`ION_PERF` set to anything but `0` or empty).
pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var(ENV_VAR).is_ok_and(|v| !v.trim().is_empty() && v.trim() != "0")
    })
}

fn since_start() -> Duration {
    START.get_or_init(Instant::now).elapsed()
}

/// Record `name` the first time it is reached. Returns the time since
/// [`begin`], or `None` when `name` was already recorded.
pub fn mark(name: &str) -> Option<Duration> {
    let elapsed = since_start();
    {
        let mut marks = MARKS.lock().unwrap_or_else(|e| e.into_inner());
        if marks.iter().any(|(n, _)| n == name) {
            return None;
        }
        marks.push((name.to_owned(), elapsed));
    }
    if enabled() {
        eprintln!("{}", line("mark", name, elapsed, unix_ms()));
    }
    Some(elapsed)
}

/// Print a repeating event (a page load, a tab opened) when enabled.
pub fn event(name: &str) {
    if enabled() {
        eprintln!("{}", line("event", name, since_start(), unix_ms()));
    }
}

/// Every mark so far, in the order they were reached.
pub fn marks() -> Vec<(String, Duration)> {
    MARKS.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

/// One stderr line. Names are trimmed to one word so the format stays parseable.
fn line(kind: &str, name: &str, elapsed: Duration, unix_ms: u128) -> String {
    let name: String = name
        .chars()
        .map(|c| if c.is_whitespace() { '-' } else { c })
        .collect();
    format!(
        "ion-perf {kind} {name} main_ms={:.1} unix_ms={unix_ms}",
        elapsed.as_secs_f64() * 1000.0
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_are_kept_once_in_order() {
        begin();
        assert!(mark("test-a").is_some());
        assert!(mark("test-b").is_some());
        assert!(mark("test-a").is_none());
        let names: Vec<String> = marks().into_iter().map(|(n, _)| n).collect();
        let a = names.iter().position(|n| n == "test-a").unwrap();
        let b = names.iter().position(|n| n == "test-b").unwrap();
        assert!(names[0] == "main" && a < b);
        assert_eq!(names.iter().filter(|n| *n == "test-a").count(), 1);
    }

    #[test]
    fn line_format_is_parseable() {
        let l = line("mark", "first frame", Duration::from_micros(412_345), 17);
        assert_eq!(l, "ion-perf mark first-frame main_ms=412.3 unix_ms=17");
    }
}
