//! Performance plumbing for Ion, free of Qt.
//!
//! - [`startup`]: named marks on the way to the first painted frame and the
//!   first loaded page, printed for `bench/bench.py` when `ION_PERF=1`.
//! - [`memory`]: how short the system is on memory (Linux PSI and
//!   `/proc/meminfo`, the macOS kernel's pressure level), with hysteresis so
//!   tab policy doesn't flap.
//! - [`process`]: how much memory Ion and its QtWebEngine helpers use.

pub mod memory;
pub mod process;
pub mod startup;
