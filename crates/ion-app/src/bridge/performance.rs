//! `Performance` QML singleton: startup marks and memory signals.
//!
//! - `mark(name)` records a startup step once (see `ion_perf::startup`);
//!   `event(name)` logs a repeating one. Both print only with `ION_PERF=1`.
//! - `memoryPressure` is `"normal"`, `"moderate"` or `"critical"`, read from
//!   the system every few seconds (`ion_perf::memory`). Tab policy can react
//!   to `onMemoryPressureChanged`, for example by unloading idle tabs.
//! - `memoryUsage()` returns the memory of Ion and its web processes as JSON:
//!   `{"totalKib": n, "processes": [{"pid", "name", "kib"}…]}`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(QString, memory_pressure, cxx_name = "memoryPressure", READ, NOTIFY)]
        #[namespace = "ion"]
        type Performance = super::PerformanceRust;

        /// Record startup step `name` the first time it is reached.
        #[qinvokable]
        fn mark(self: &Performance, name: &QString);

        /// Log a repeating event (a page load) for the benchmark.
        #[qinvokable]
        fn event(self: &Performance, name: &QString);

        /// Memory of Ion's process tree as JSON.
        #[qinvokable]
        #[cxx_name = "memoryUsage"]
        fn memory_usage(self: &Performance) -> QString;
    }

    impl cxx_qt::Threading for Performance {}
    impl cxx_qt::Initialize for Performance {}
}

use std::pin::Pin;
use std::time::Duration;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use ion_perf::memory::{self, Monitor, Pressure};

/// How often the system's memory is read. PSI's `avg10` is a 10 s average,
/// so reading more often adds nothing.
const POLL: Duration = Duration::from_secs(5);

pub struct PerformanceRust {
    memory_pressure: QString,
}

impl Default for PerformanceRust {
    fn default() -> Self {
        Self {
            memory_pressure: QString::from(Pressure::Normal.as_str()),
        }
    }
}

impl cxx_qt::Initialize for qobject::Performance {
    fn initialize(self: Pin<&mut Self>) {
        let thread = self.qt_thread();
        let spawned = std::thread::Builder::new()
            .name("ion-memory".into())
            .spawn(move || {
                let mut monitor = Monitor::default();
                loop {
                    if let Some(level) = memory::read().and_then(|r| monitor.update(&r)) {
                        let queued = thread.queue(move |mut performance| {
                            // Read-only for QML: set the field, then notify.
                            performance.as_mut().rust_mut().memory_pressure =
                                QString::from(level.as_str());
                            performance.memory_pressure_changed();
                        });
                        // The singleton is gone: Ion is quitting.
                        if queued.is_err() {
                            return;
                        }
                    }
                    std::thread::sleep(POLL);
                }
            });
        if let Err(e) = spawned {
            eprintln!("ion: cannot watch memory pressure: {e}");
        }
    }
}

impl qobject::Performance {
    fn mark(&self, name: &QString) {
        ion_perf::startup::mark(&name.to_string());
    }

    fn event(&self, name: &QString) {
        ion_perf::startup::event(&name.to_string());
    }

    fn memory_usage(&self) -> QString {
        let tree = ion_perf::process::tree(std::process::id());
        let total: u64 = tree.iter().map(|p| p.kib).sum();
        let processes: Vec<serde_json::Value> = tree
            .iter()
            .map(|p| serde_json::json!({ "pid": p.pid, "name": p.name, "kib": p.kib }))
            .collect();
        QString::from(
            serde_json::json!({ "totalKib": total, "processes": processes })
                .to_string()
                .as_str(),
        )
    }
}
