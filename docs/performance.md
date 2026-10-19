# Performance

What keeps Ion fast, how to measure it, and what still needs checking on real
hardware.

## Measuring

`bench/bench.py` launches a browser against its own local test pages (no
network needed) and reports:

| Test | What it measures |
|---|---|
| startup | launch until the test page's `load` event, cold (fresh profile) and warm; for Ion also launch until the first painted frame |
| memory | the whole process tree (PSS on Linux, RSS on macOS) launched with 1 tab and with 10 tabs, and the cost of each extra tab |
| restore | Ion only: first frame and first page when reopening a saved session of 30 tabs |
| scroll | frames per second, long frames and 95th-percentile frame time while a heavy page scrolls smoothly |

```sh
nix develop
cargo build --release -p ion-app
bench/bench.py                                    # Ion, on your desktop's GPU
bench/bench.py --browser chromium --binary "$(command -v chromium)"
bench/bench.py --browser firefox --binary "$(command -v firefox)"
bench/bench.py --headless                         # Xvfb, software rendering (CI-like)
bench/bench.py --only startup --runs 10 --json out.json
```

Every number is a median over `--runs`. Compare runs from the same machine
only, with nothing else busy.

Ion prints its own startup marks with `ION_PERF=1`:

```text
ion-perf mark main main_ms=0.0 …
ion-perf mark engine main_ms=0.2 …        QtWebEngine initialised
ion-perf mark app main_ms=11.5 …          QGuiApplication, config, safety model
ion-perf mark window main_ms=140.6 …      Main.qml built
ion-perf mark first-frame main_ms=146.2 … the window is on screen
ion-perf mark first-page main_ms=390.0 …  the first page finished loading
```

QML-level detail comes from `qmlprofiler` (in the dev shell). It needs QML
debugging compiled in: add `QQmlDebuggingEnabler::enableDebugging(true);` at
the top of `initializeWebEngine()` in `cpp/webengine.cpp` for the profiling
build only, then `qmlprofiler --output trace.qtd target/release/ion URL`.

## What Ion does

- **Window first, engine second.** Opening the browser profile starts
  Chromium's storage and network service and is the slowest step of startup
  (about 80–120 ms). `Main.qml` creates the profile right after the window's
  first frame (`startEngine()`), so the window appears that much sooner. The
  page starts loading a frame later than before, which is not measurable.
- **Restored tabs cost almost nothing until shown.** Restoring a session
  loads only the current tab; the others load when first shown, as in Chrome
  and Firefox. Each tab's per-page UI (new-tab page, find bar, prompts,
  dialogs, error page, context menu, screenshot tool) is created by a
  `TabPart` once the tab is live (`BrowserTab.live`), so an unopened restored
  tab is a bare web view.
- **The new-tab page exists only while it shows.** Before, every tab kept one
  (with its tiles) and rebuilt all of them on every config change.
- **Hardware video on Linux.** VA-API decode and encode features are switched
  on (`crates/ion-platform/src/chromium.rs`); see
  [platform.md](platform.md#hardware-video-and-gpu-linux) for the checks.

## Memory pressure

`Performance.memoryPressure` (QML singleton, `bridge/performance.rs`) is
`"normal"`, `"moderate"` or `"critical"`, read every 5 s from the system:

- Linux: pressure stall information (`/proc/pressure/memory`) and
  `MemAvailable`. Moderate: under 12% available, or tasks stalled on memory
  more than 20% of the last 10 s. Critical: under 5% available, or every task
  stalled more than 10% of the time.
- macOS: the kernel's own level (`kern.memorystatus_vm_pressure_level`).

It rises at once and falls only after two calm readings, so a policy built on
it doesn't flap. Tab policy (unloading idle tabs, owned by the tabs code)
reacts to `onMemoryPressureChanged`. `Performance.memoryUsage()` returns the
memory of Ion and every web process as JSON, for a task-manager style view.

## Results

Linux x86_64, 4 CPUs, Xvfb with software rendering (`--headless`), so GPU
work runs on the CPU for every browser alike. Medians of 5 runs.

| | Ion before (main) | Ion after | Chromium 154 | Firefox 157 |
|---|---|---|---|---|
| Launch → window on screen | 392 ms | **212 ms** | – | – |
| Launch → page loaded, cold | 503 ms | 517 ms | 432 ms | 1626 ms |
| Launch → page loaded, warm | 511 ms | 508 ms | 463 ms | 1482 ms |
| Memory, 1 tab | 336 MiB | 335 MiB | 466 MiB | 529 MiB |
| Memory, 10 tabs | 636 MiB | 630 MiB | 704 MiB | 711 MiB |
| Each extra tab | 33 MiB | 33 MiB | 27 MiB | 20 MiB |
| Restore 30 tabs → window | 662 ms | **242 ms** | – | – |
| Restore 30 tabs → page loaded | 1559 ms | **834 ms** | – | – |
| Restore 30 tabs, memory | 1298 MiB | **721 MiB** | – | – |
| Scrolling (software, 60 Hz) | 59.9 fps | 59.9 fps | 59.9 fps | 59.0 fps |

Before this work a restored session loaded every tab at startup (the views
were created after the restore flag had already cleared), which is where
most of the restore time and memory went. Scrolling is capped by Xvfb's
60 Hz; it only tells something on real hardware.

## To check on real hardware

The cloud CI has no GPU, compositor or Mac. On a real machine, run
`bench/bench.py` without `--headless` for each browser, and:

1. The [platform.md GPU checklist](platform.md#hardware-video-and-gpu-linux):
   `chrome://gpu` shows hardware compositing, rasterization and video decode.
2. Scrolling: `bench/bench.py --only scroll` should hold the display's refresh
   rate (60, 120 or 144 fps) with under 2% long frames.
3. Fractional scaling (Wayland, 125% or 150%): text and the page are sharp,
   not blurred, and the page fills the window without a gap.
