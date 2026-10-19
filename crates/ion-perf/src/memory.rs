//! How short the system is on memory.
//!
//! Chrome, Firefox and Safari unload background tabs when the system runs low
//! on memory. QtWebEngine has no such policy, so Ion reads the pressure itself:
//!
//! - Linux: pressure stall information (`/proc/pressure/memory`, the share of
//!   time tasks waited on memory) plus `MemAvailable` from `/proc/meminfo`.
//! - macOS: the kernel's own level (`kern.memorystatus_vm_pressure_level`),
//!   the same signal Safari reacts to.
//!
//! [`Monitor`] turns readings into a level with hysteresis: it rises at once
//! but only falls after two calm readings in a row, so a policy built on it
//! doesn't flap.

/// How short the system is on memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Pressure {
    #[default]
    Normal,
    /// Worth freeing memory that is cheap to get back (long-idle tabs).
    Moderate,
    /// The system is swapping or about to kill processes; free what can be.
    Critical,
}

impl Pressure {
    pub fn as_str(self) -> &'static str {
        match self {
            Pressure::Normal => "normal",
            Pressure::Moderate => "moderate",
            Pressure::Critical => "critical",
        }
    }
}

/// What the system reported. Unknown fields are `None`.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Reading {
    pub total_kib: Option<u64>,
    pub available_kib: Option<u64>,
    /// Linux PSI `some avg10`: percent of the last 10 s in which at least one
    /// task stalled on memory.
    pub some_avg10: Option<f64>,
    /// Linux PSI `full avg10`: percent in which every task stalled.
    pub full_avg10: Option<f64>,
    /// macOS `kern.memorystatus_vm_pressure_level`: 1 normal, 2 warn, 4 critical.
    pub kernel_level: Option<u32>,
}

impl Reading {
    /// The level this reading alone suggests.
    pub fn pressure(&self) -> Pressure {
        if let Some(level) = self.kernel_level {
            return match level {
                4.. => Pressure::Critical,
                2 | 3 => Pressure::Moderate,
                _ => Pressure::Normal,
            };
        }
        let available_percent = match (self.available_kib, self.total_kib) {
            (Some(available), Some(total)) if total > 0 => available as f64 * 100.0 / total as f64,
            _ => 100.0,
        };
        let some = self.some_avg10.unwrap_or(0.0);
        let full = self.full_avg10.unwrap_or(0.0);
        if available_percent < 5.0 || full >= 10.0 {
            Pressure::Critical
        } else if available_percent < 12.0 || some >= 20.0 || full >= 2.0 {
            Pressure::Moderate
        } else {
            Pressure::Normal
        }
    }
}

/// Levels over time, rising at once and falling after two calm readings.
#[derive(Debug, Default)]
pub struct Monitor {
    level: Pressure,
    calmer: u8,
}

impl Monitor {
    pub fn level(&self) -> Pressure {
        self.level
    }

    /// Feed one reading; returns the new level when it changed.
    pub fn update(&mut self, reading: &Reading) -> Option<Pressure> {
        let seen = reading.pressure();
        if seen > self.level {
            self.level = seen;
            self.calmer = 0;
            return Some(seen);
        }
        if seen == self.level {
            self.calmer = 0;
            return None;
        }
        self.calmer += 1;
        if self.calmer < 2 {
            return None;
        }
        self.calmer = 0;
        self.level = seen;
        Some(seen)
    }
}

/// `MemTotal` and `MemAvailable` from `/proc/meminfo`, in KiB.
pub fn parse_meminfo(text: &str) -> (Option<u64>, Option<u64>) {
    let field = |key: &str| {
        text.lines().find_map(|line| {
            let rest = line.strip_prefix(key)?.strip_prefix(':')?;
            rest.split_whitespace().next()?.parse().ok()
        })
    };
    (field("MemTotal"), field("MemAvailable"))
}

/// `some avg10` and `full avg10` from `/proc/pressure/memory`.
pub fn parse_psi(text: &str) -> (Option<f64>, Option<f64>) {
    let avg10 = |kind: &str| {
        text.lines().find_map(|line| {
            let mut words = line.split_whitespace();
            if words.next()? != kind {
                return None;
            }
            words.find_map(|w| w.strip_prefix("avg10=")?.parse().ok())
        })
    };
    (avg10("some"), avg10("full"))
}

/// Read the system now. `None` where Ion can't tell (another OS, no `/proc`).
pub fn read() -> Option<Reading> {
    read_system()
}

#[cfg(target_os = "linux")]
fn read_system() -> Option<Reading> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
    let (total_kib, available_kib) = parse_meminfo(&meminfo);
    let (some_avg10, full_avg10) = std::fs::read_to_string("/proc/pressure/memory")
        .map(|t| parse_psi(&t))
        .unwrap_or_default();
    Some(Reading {
        total_kib,
        available_kib,
        some_avg10,
        full_avg10,
        kernel_level: None,
    })
}

#[cfg(target_os = "macos")]
fn read_system() -> Option<Reading> {
    fn sysctl<T: Default + Copy>(name: &std::ffi::CStr) -> Option<T> {
        let mut value = T::default();
        let mut size = std::mem::size_of::<T>();
        // SAFETY: `value` is a plain integer of `size` bytes; sysctlbyname
        // writes at most `size` bytes and reports how many it wrote.
        let rc = unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                (&mut value as *mut T).cast(),
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        (rc == 0 && size == std::mem::size_of::<T>()).then_some(value)
    }
    let level: Option<u32> = sysctl(c"kern.memorystatus_vm_pressure_level");
    let total: Option<u64> = sysctl(c"hw.memsize");
    Some(Reading {
        total_kib: total.map(|b| b / 1024),
        kernel_level: level,
        ..Reading::default()
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn read_system() -> Option<Reading> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEMINFO: &str =
        "MemTotal:       16000000 kB\nMemFree:          500000 kB\nMemAvailable:    1000000 kB\n";
    const PSI: &str = "some avg10=25.50 avg60=3.00 avg300=1.00 total=123\nfull avg10=1.25 avg60=0.10 avg300=0.00 total=45\n";

    #[test]
    fn parses_proc_files() {
        assert_eq!(parse_meminfo(MEMINFO), (Some(16_000_000), Some(1_000_000)));
        assert_eq!(parse_psi(PSI), (Some(25.5), Some(1.25)));
        assert_eq!(parse_psi(""), (None, None));
    }

    fn linux(available_percent: u64, some: f64, full: f64) -> Reading {
        Reading {
            total_kib: Some(100_000),
            available_kib: Some(available_percent * 1000),
            some_avg10: Some(some),
            full_avg10: Some(full),
            kernel_level: None,
        }
    }

    #[test]
    fn levels_from_linux_readings() {
        assert_eq!(linux(50, 0.0, 0.0).pressure(), Pressure::Normal);
        assert_eq!(linux(10, 0.0, 0.0).pressure(), Pressure::Moderate);
        assert_eq!(linux(50, 30.0, 0.0).pressure(), Pressure::Moderate);
        assert_eq!(linux(3, 0.0, 0.0).pressure(), Pressure::Critical);
        assert_eq!(linux(50, 60.0, 15.0).pressure(), Pressure::Critical);
        assert_eq!(Reading::default().pressure(), Pressure::Normal);
    }

    #[test]
    fn levels_from_macos_kernel() {
        let mac = |level| Reading {
            kernel_level: Some(level),
            ..Reading::default()
        };
        assert_eq!(mac(1).pressure(), Pressure::Normal);
        assert_eq!(mac(2).pressure(), Pressure::Moderate);
        assert_eq!(mac(4).pressure(), Pressure::Critical);
    }

    #[test]
    fn monitor_rises_at_once_and_falls_slowly() {
        let mut m = Monitor::default();
        assert_eq!(m.update(&linux(50, 0.0, 0.0)), None);
        assert_eq!(m.update(&linux(3, 0.0, 0.0)), Some(Pressure::Critical));
        assert_eq!(m.update(&linux(50, 0.0, 0.0)), None);
        assert_eq!(m.update(&linux(3, 0.0, 0.0)), None);
        assert_eq!(m.update(&linux(50, 0.0, 0.0)), None);
        assert_eq!(m.update(&linux(10, 0.0, 0.0)), Some(Pressure::Moderate));
        assert_eq!(m.level(), Pressure::Moderate);
    }

    #[test]
    fn reads_this_system() {
        if cfg!(any(target_os = "linux", target_os = "macos")) {
            assert!(read().is_some());
        }
    }
}
