#!/usr/bin/env python3
"""Repeatable speed and memory benchmark for Ion (and, for comparison,
Chromium and Firefox).

It serves its own test pages from a local HTTP server, so no network is
needed, and the pages report back to that server. That makes every number
below comparable across browsers:

  startup     launch -> the test page's `load` event reaches the server,
              cold (fresh profile) and warm (second launch of a profile).
              For Ion also launch -> first painted frame, from Ion's own
              `ION_PERF=1` marks.
  memory      total memory of the browser's process tree (PSS on Linux,
              RSS on macOS) launched with 1 tab, then launched with --tabs
              tabs, and the cost of each extra tab.
  restore     Ion only: launch -> first frame and first page when Ion
              reopens a saved session of --restore-tabs tabs, and the memory
              in use once it settles.
  scroll      frames per second and long frames while a heavy page scrolls
              smoothly, measured with requestAnimationFrame in the page.

Usage (inside `nix develop`, after `cargo build --release -p ion-app`):

  bench/bench.py                         # Ion from target/release/ion
  bench/bench.py --ion result/bin/ion    # a `nix build`
  bench/bench.py --browser chromium --binary "$(command -v chromium)"
  bench/bench.py --headless              # in Xvfb, software rendering
  bench/bench.py --json out.json         # also save the numbers

On a desktop session, run without --headless to measure the real GPU path.
Results vary with the machine; compare runs from the same machine only.
"""

from __future__ import annotations

import argparse
import http.server
import json
import os
import platform
import shutil
import signal
import socketserver
import statistics
import subprocess
import sys
import tempfile
import threading
import time
import urllib.parse
from pathlib import Path

HERE = Path(__file__).resolve().parent
PAGES = HERE / "pages"


# ---------------------------------------------------------------- server ---

class Reports:
    """Reports the pages POST/GET to /report, with their arrival time."""

    def __init__(self) -> None:
        self.cond = threading.Condition()
        self.items: list[dict] = []

    def add(self, item: dict) -> None:
        with self.cond:
            item["arrived"] = time.time()
            self.items.append(item)
            self.cond.notify_all()

    def wait(self, kind: str, count: int = 1, timeout: float = 60.0, since: float = 0.0) -> list[dict]:
        deadline = time.time() + timeout
        with self.cond:
            while True:
                found = [i for i in self.items if i.get("kind") == kind and i["arrived"] >= since]
                if len(found) >= count:
                    return found
                left = deadline - time.time()
                if left <= 0:
                    raise TimeoutError(f"waited {timeout:.0f}s for {count} '{kind}' report(s), got {len(found)}")
                self.cond.wait(left)


def serve(reports: Reports) -> tuple[socketserver.TCPServer, int]:
    class Handler(http.server.SimpleHTTPRequestHandler):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, directory=str(PAGES), **kwargs)

        def log_message(self, *args):  # quiet
            pass

        def end_headers(self):
            # Every run measures the same page load, not a cache hit.
            self.send_header("Cache-Control", "no-store")
            super().end_headers()

        def do_POST(self):
            if not self.path.startswith("/report"):
                self.send_error(404)
                return
            length = int(self.headers.get("Content-Length") or 0)
            try:
                reports.add(json.loads(self.rfile.read(length) or b"{}"))
            except json.JSONDecodeError:
                pass
            self.send_response(204)
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()

    socketserver.TCPServer.allow_reuse_address = True
    server = socketserver.ThreadingTCPServer(("127.0.0.1", 0), Handler)
    server.daemon_threads = True
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server, server.server_address[1]


# --------------------------------------------------------------- memory ---

def _linux_tree(root: int) -> list[int]:
    parents: dict[int, int] = {}
    for entry in os.listdir("/proc"):
        if not entry.isdigit():
            continue
        try:
            stat = Path(f"/proc/{entry}/stat").read_text()
        except OSError:
            continue
        parents[int(entry)] = int(stat[stat.rfind(")") + 2:].split()[1])
    tree, frontier = [root], [root]
    while frontier:
        parent = frontier.pop()
        children = [p for p, pp in parents.items() if pp == parent]
        tree += children
        frontier += children
    return tree


def _linux_kib(pid: int) -> int:
    """PSS, or RSS where PSS isn't readable (as in ion_perf::process)."""
    try:
        for line in Path(f"/proc/{pid}/smaps_rollup").read_text().splitlines():
            if line.startswith("Pss:"):
                return int(line.split()[1])
    except OSError:
        pass
    try:
        resident_pages = int(Path(f"/proc/{pid}/statm").read_text().split()[1])
        return resident_pages * os.sysconf("SC_PAGE_SIZE") // 1024
    except (OSError, IndexError, ValueError):
        return 0


def tree_memory_mib(root: int) -> tuple[float, int]:
    """Total memory of `root` and its descendants in MiB, and process count."""
    if sys.platform.startswith("linux"):
        pids = _linux_tree(root)
        return sum(_linux_kib(p) for p in pids) / 1024, len(pids)
    out = subprocess.run(["ps", "-A", "-o", "pid=,ppid=,rss="], capture_output=True, text=True).stdout
    rows = [tuple(int(x) for x in line.split()[:3]) for line in out.splitlines() if line.strip()]
    tree, frontier = {root}, [root]
    while frontier:
        parent = frontier.pop()
        for pid, ppid, _ in rows:
            if ppid == parent and pid not in tree:
                tree.add(pid)
                frontier.append(pid)
    return sum(rss for pid, _, rss in rows if pid in tree) / 1024, len(tree)


def settled_memory(root: int, seconds: float = 4.0) -> tuple[float, int]:
    """Memory once it stops moving: the last of a few samples."""
    time.sleep(seconds)
    samples = []
    for _ in range(3):
        samples.append(tree_memory_mib(root))
        time.sleep(0.5)
    return samples[-1]


# -------------------------------------------------------------- browsers ---

class Browser:
    name = "browser"

    def __init__(self, binary: str, headless: bool) -> None:
        self.binary = binary
        self.headless = headless
        self.profile = Path(tempfile.mkdtemp(prefix=f"ion-bench-{self.name}-"))
        self.proc: subprocess.Popen | None = None
        self.stderr_lines: list[str] = []

    def env(self) -> dict:
        return dict(os.environ)

    def launch_args(self, *urls: str) -> list[str]:
        raise NotImplementedError

    def fresh_profile(self) -> None:
        shutil.rmtree(self.profile, ignore_errors=True)
        self.profile.mkdir(parents=True)
        self.prepare_profile()

    def prepare_profile(self) -> None:
        pass

    def start(self, url: str) -> float:
        return self.start_many([url])

    def start_many(self, urls: list[str]) -> float:
        started = time.time()
        self.stderr_lines = []
        self.proc = subprocess.Popen(
            self.launch_args(*urls), env=self.env(),
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True,
            start_new_session=True,
        )
        threading.Thread(target=self._drain, args=(self.proc,), daemon=True).start()
        return started

    def _drain(self, proc: subprocess.Popen) -> None:
        for line in proc.stderr:
            self.stderr_lines.append(line.rstrip("\n"))

    def stop(self) -> None:
        if not self.proc:
            return
        # SIGTERM to the browser alone quits it the normal way, so the next
        # run doesn't start as a crash recovery; its helpers exit with it.
        try:
            self.proc.send_signal(signal.SIGTERM)
            self.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            pass
        try:
            os.killpg(self.proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        self.proc.wait()
        self.proc = None
        time.sleep(0.5)

    def marks(self) -> dict[str, int]:
        return {}


class Ion(Browser):
    name = "ion"

    def env(self) -> dict:
        env = dict(os.environ)
        env.update({
            "ION_PERF": "1",
            "ION_DATA_DIR": str(self.profile / "data"),
            "ION_CONFIG_DIR": str(self.profile / "config"),
            "XDG_RUNTIME_DIR": str(self.profile / "run"),
            "QT_FORCE_STDERR_LOGGING": "1",
        })
        if self.headless:
            env.update({
                "QT_QUICK_BACKEND": "software",
                "QT_XCB_GL_INTEGRATION": "none",
                "QTWEBENGINE_CHROMIUM_FLAGS": "--disable-gpu --no-sandbox",
                "QT_QPA_PLATFORM": "xcb",
            })
        return env

    def prepare_profile(self) -> None:
        for sub in ("data", "config", "run"):
            (self.profile / sub).mkdir(exist_ok=True)
        os.chmod(self.profile / "run", 0o700)
        # Each run opens exactly the page it is given.
        (self.profile / "config" / "config.toml").write_text(
            "[general]\nrestoreSession = false\n"
        )

    def restore_sessions(self, on: bool) -> None:
        (self.profile / "config" / "config.toml").write_text(
            f"[general]\nrestoreSession = {'true' if on else 'false'}\n"
        )

    def launch_args(self, *urls: str) -> list[str]:
        return [self.binary, *urls]

    def marks(self) -> dict[str, int]:
        """Ion's own startup marks, as unix milliseconds."""
        out = {}
        for line in self.stderr_lines:
            parts = line.split()
            if len(parts) == 5 and parts[0] == "ion-perf" and parts[1] == "mark":
                out[parts[2]] = int(parts[4].removeprefix("unix_ms="))
        return out


class Chromium(Browser):
    name = "chromium"

    def launch_args(self, *urls: str) -> list[str]:
        args = [self.binary, f"--user-data-dir={self.profile}", "--no-first-run",
                "--no-default-browser-check", "--disable-sync", "--password-store=basic"]
        if self.headless:
            args += ["--disable-gpu", "--no-sandbox", "--ozone-platform=x11"]
        return args + list(urls)


class Firefox(Browser):
    name = "firefox"

    def env(self) -> dict:
        env = dict(os.environ)
        env["MOZ_ENABLE_WAYLAND"] = "0" if self.headless else env.get("MOZ_ENABLE_WAYLAND", "1")
        return env

    def prepare_profile(self) -> None:
        (self.profile / "user.js").write_text("\n".join([
            'user_pref("browser.shell.checkDefaultBrowser", false);',
            'user_pref("browser.startup.homepage_override.mstone", "ignore");',
            'user_pref("startup.homepage_welcome_url", "");',
            'user_pref("datareporting.policy.dataSubmissionEnabled", false);',
            'user_pref("toolkit.telemetry.reportingpolicy.firstRun", false);',
            'user_pref("browser.aboutwelcome.enabled", false);',
            'user_pref("trailhead.firstrun.didSeeAboutWelcome", true);',
            'user_pref("browser.tabs.warnOnClose", false);',
        ]) + "\n")

    def launch_args(self, *urls: str) -> list[str]:
        return [self.binary, "--profile", str(self.profile), "--no-remote", *urls]


BROWSERS = {"ion": Ion, "chromium": Chromium, "firefox": Firefox}


# ----------------------------------------------------------------- tests ---

def page_url(port: int, page: str, **query) -> str:
    query.setdefault("t", f"{time.time():.6f}")
    return f"http://127.0.0.1:{port}/{page}?{urllib.parse.urlencode(query)}"


def bench_startup(browser: Browser, reports: Reports, port: int, runs: int) -> dict:
    cold, warm, frame_cold, frame_warm = [], [], [], []
    for run in range(runs):
        browser.fresh_profile()
        for kind, out, frames in (("cold", cold, frame_cold), ("warm", warm, frame_warm)):
            url = page_url(port, "article.html", run=run, kind=kind)
            started = browser.start(url)
            try:
                report = reports.wait("load", timeout=90, since=started)[-1]
                out.append((report["arrived"] - started) * 1000)
                marks = browser.marks()
                if "first-frame" in marks:
                    frames.append(marks["first-frame"] - started * 1000)
            finally:
                # Let the profile finish writing before the next launch reuses it.
                time.sleep(1.5)
                browser.stop()
        print(f"  startup run {run + 1}/{runs}: cold {cold[-1]:.0f} ms, warm {warm[-1]:.0f} ms", flush=True)
    result = {"cold_ms": statistics.median(cold), "warm_ms": statistics.median(warm)}
    if frame_cold:
        result["first_frame_cold_ms"] = statistics.median(frame_cold)
        result["first_frame_warm_ms"] = statistics.median(frame_warm)
    return result


def bench_memory(browser: Browser, reports: Reports, port: int, tabs: int) -> dict:
    """One launch with one tab, one with `tabs` tabs (all on the command line,
    which every browser supports the same way)."""
    measured = {}
    for count in (1, tabs):
        browser.fresh_profile()
        urls = [page_url(port, "article.html", tab=i) for i in range(count)]
        started = browser.start_many(urls)
        try:
            reports.wait("load", count=count, timeout=120, since=started)
            measured[count] = settled_memory(browser.proc.pid, seconds=4 + count * 0.3)
        finally:
            browser.stop()
        mib, procs = measured[count]
        print(f"  memory: {count} tab(s) {mib:.0f} MiB ({procs} processes)", flush=True)
    one, many = measured[1][0], measured[tabs][0]
    return {
        "one_tab_mib": round(one, 1),
        "tabs": tabs,
        "many_tabs_mib": round(many, 1),
        "per_extra_tab_mib": round((many - one) / max(1, tabs - 1), 1),
        "processes": measured[tabs][1],
    }


def bench_restore(browser: Browser, reports: Reports, port: int, runs: int, tabs: int) -> dict:
    """Ion only: start with a saved session of `tabs` tabs. Only the current
    tab loads; the rest wait until shown, so this measures what restoring
    costs before the page appears."""
    if not isinstance(browser, Ion):
        return {}
    browser.fresh_profile()
    urls = [page_url(port, "article.html", tab=i) for i in range(tabs)]
    started = browser.start_many(urls)
    try:
        reports.wait("load", count=tabs, timeout=120, since=started)
        time.sleep(2.5)  # the session is saved a second after it changes
    finally:
        browser.stop()
    browser.restore_sessions(True)
    frame, page, memory = [], [], []
    for run in range(runs):
        started = browser.start_many([])
        try:
            # The tab that was current when the session was saved: the first.
            deadline = time.time() + 90
            while True:
                loads = reports.wait("load", timeout=max(1, deadline - time.time()), since=started)
                current = [r for r in loads if "tab=0&" in r.get("query", "")]
                if current:
                    break
                time.sleep(0.1)
            page.append((current[0]["arrived"] - started) * 1000)
            memory.append(settled_memory(browser.proc.pid)[0])
            marks = browser.marks()
            if "first-frame" in marks:
                frame.append(marks["first-frame"] - started * 1000)
        finally:
            browser.stop()
        shown = f"first frame {frame[-1]:.0f} ms, " if frame else ""
        print(f"  restore run {run + 1}/{runs}: {shown}page {page[-1]:.0f} ms, "
              f"memory {memory[-1]:.0f} MiB", flush=True)
    result = {"tabs": tabs, "first_page_ms": statistics.median(page),
              "memory_mib": round(statistics.median(memory), 1)}
    if frame:
        result["first_frame_ms"] = statistics.median(frame)
    return result


def bench_scroll(browser: Browser, reports: Reports, port: int, runs: int) -> dict:
    fps, long_frames, p95 = [], [], []
    browser.fresh_profile()
    for run in range(runs):
        started = browser.start(page_url(port, "scroll.html", run=run))
        try:
            report = reports.wait("scroll", timeout=120, since=started)[-1]
        finally:
            browser.stop()
        fps.append(report["fps"])
        long_frames.append(report["longFramesPercent"])
        p95.append(report["p95Ms"])
        print(f"  scroll run {run + 1}/{runs}: {report['fps']:.1f} fps, "
              f"{report['longFramesPercent']:.1f}% long frames, p95 {report['p95Ms']:.1f} ms", flush=True)
    return {
        "fps": round(statistics.median(fps), 1),
        "long_frames_percent": round(statistics.median(long_frames), 1),
        "p95_frame_ms": round(statistics.median(p95), 1),
    }


# ------------------------------------------------------------------ main ---

def default_ion() -> str:
    root = HERE.parent
    for candidate in (root / "target/release/ion", root / "result/bin/ion", root / "target/debug/ion"):
        if candidate.exists():
            return str(candidate)
    return shutil.which("ion") or "ion"


def start_xvfb() -> subprocess.Popen | None:
    xvfb = shutil.which("Xvfb")
    if not xvfb:
        sys.exit("--headless needs Xvfb on PATH (nix shell nixpkgs#xorg.xvfb)")
    display = ":97"
    proc = subprocess.Popen([xvfb, display, "-screen", "0", "1440x900x24", "-nolisten", "tcp"],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    os.environ["DISPLAY"] = display
    os.environ.pop("WAYLAND_DISPLAY", None)
    time.sleep(1)
    return proc


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--browser", choices=BROWSERS, default="ion")
    parser.add_argument("--binary", "--ion", dest="binary", help="browser executable")
    parser.add_argument("--runs", type=int, default=5, help="startup and scroll runs (median is reported)")
    parser.add_argument("--tabs", type=int, default=10, help="tabs for the memory test")
    parser.add_argument("--only", choices=["startup", "memory", "restore", "scroll"], action="append",
                        help="run only these tests (repeatable)")
    parser.add_argument("--restore-tabs", type=int, default=30, help="tabs in the saved session (Ion only)")
    parser.add_argument("--headless", action="store_true", help="run in Xvfb with software rendering")
    parser.add_argument("--label", help="name for this run in the output")
    parser.add_argument("--json", help="write results to this file")
    args = parser.parse_args()

    binary = args.binary or (default_ion() if args.browser == "ion" else shutil.which(args.browser))
    if not binary:
        sys.exit(f"cannot find {args.browser}; pass --binary")
    xvfb = start_xvfb() if args.headless else None
    reports = Reports()
    server, port = serve(reports)
    browser = BROWSERS[args.browser](binary, args.headless)
    tests = args.only or ["startup", "memory", "restore", "scroll"]
    results: dict = {
        "label": args.label or args.browser,
        "browser": args.browser,
        "binary": binary,
        "machine": f"{platform.system()} {platform.machine()}, {os.cpu_count()} CPUs",
        "headless": args.headless,
    }
    try:
        print(f"Benchmarking {args.browser} ({binary})", flush=True)
        if "startup" in tests:
            results["startup"] = bench_startup(browser, reports, port, args.runs)
        if "memory" in tests:
            results["memory"] = bench_memory(browser, reports, port, args.tabs)
        if "restore" in tests and args.browser == "ion":
            results["restore"] = bench_restore(browser, reports, port, args.runs, args.restore_tabs)
        if "scroll" in tests:
            results["scroll"] = bench_scroll(browser, reports, port, args.runs)
    finally:
        browser.stop()
        server.shutdown()
        shutil.rmtree(browser.profile, ignore_errors=True)
        if xvfb:
            xvfb.terminate()
    print(json.dumps(results, indent=2))
    if args.json:
        Path(args.json).write_text(json.dumps(results, indent=2) + "\n")


if __name__ == "__main__":
    main()
