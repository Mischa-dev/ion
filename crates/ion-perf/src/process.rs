//! Memory used by a process and everything it started.
//!
//! Ion's own process holds the UI, the network service and (with
//! QtWebEngine's in-process GPU) the GPU thread; each site runs in a
//! `QtWebEngineProcess` child. The fair total is the sum over the tree.
//!
//! On Linux each process counts its proportional set size (PSS), which splits
//! shared pages between the processes sharing them, so the sum doesn't count
//! Qt's libraries once per renderer. macOS reports resident size (RSS), which
//! overstates the total by the shared libraries.

/// One process of the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessMemory {
    pub pid: u32,
    pub parent: u32,
    pub name: String,
    pub kib: u64,
}

/// `root` and all its descendants, root first.
pub fn tree(root: u32) -> Vec<ProcessMemory> {
    let mut tree = descendants(root, list());
    // Measuring is the slow part (PSS walks the page tables), so only the
    // tree's processes are measured.
    for process in &mut tree {
        if process.kib == 0 {
            process.kib = measure(process.pid).unwrap_or(0);
        }
    }
    tree
}

/// The sum of [`tree`], in KiB.
pub fn total_kib(root: u32) -> u64 {
    tree(root).iter().map(|p| p.kib).sum()
}

/// Keep `root` and the processes below it.
pub fn descendants(root: u32, all: Vec<ProcessMemory>) -> Vec<ProcessMemory> {
    let mut keep: Vec<ProcessMemory> = Vec::new();
    let mut frontier = vec![root];
    let mut rest = all;
    if let Some(i) = rest.iter().position(|p| p.pid == root) {
        keep.push(rest.swap_remove(i));
    }
    while let Some(parent) = frontier.pop() {
        let (children, others): (Vec<_>, Vec<_>) =
            rest.into_iter().partition(|p| p.parent == parent);
        rest = others;
        frontier.extend(children.iter().map(|p| p.pid));
        keep.extend(children);
    }
    keep
}

/// Name and parent pid from `/proc/<pid>/stat`. The name may hold spaces or
/// parentheses, so it runs to the last `)`.
pub fn parse_stat(text: &str) -> Option<(String, u32)> {
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    let name = text.get(open + 1..close)?.to_owned();
    let mut rest = text.get(close + 1..)?.split_whitespace();
    let _state = rest.next()?;
    let parent = rest.next()?.parse().ok()?;
    Some((name, parent))
}

/// `Pss:` from `/proc/<pid>/smaps_rollup`, in KiB.
pub fn parse_smaps_rollup(text: &str) -> Option<u64> {
    text.lines().find_map(|line| {
        let rest = line.strip_prefix("Pss:")?;
        rest.split_whitespace().next()?.parse().ok()
    })
}

/// Rows of `ps -A -o pid=,ppid=,rss=,comm=` (RSS in KiB).
pub fn parse_ps(text: &str) -> Vec<ProcessMemory> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let pid = words.next()?.parse().ok()?;
            let parent = words.next()?.parse().ok()?;
            let kib = words.next()?.parse().ok()?;
            let command = words.collect::<Vec<_>>().join(" ");
            let name = command.rsplit('/').next().unwrap_or_default().to_owned();
            Some(ProcessMemory {
                pid,
                parent,
                name,
                kib,
            })
        })
        .collect()
}

/// Every process with its parent; memory is filled in by [`measure`].
#[cfg(target_os = "linux")]
fn list() -> Vec<ProcessMemory> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
            let stat = std::fs::read_to_string(entry.path().join("stat")).ok()?;
            let (name, parent) = parse_stat(&stat)?;
            Some(ProcessMemory {
                pid,
                parent,
                name,
                kib: 0,
            })
        })
        .collect()
}

/// PSS, or RSS where PSS isn't readable (another user's process).
#[cfg(target_os = "linux")]
fn measure(pid: u32) -> Option<u64> {
    let dir = std::path::Path::new("/proc").join(pid.to_string());
    if let Some(pss) = std::fs::read_to_string(dir.join("smaps_rollup"))
        .ok()
        .and_then(|t| parse_smaps_rollup(&t))
    {
        return Some(pss);
    }
    // SAFETY: sysconf has no preconditions.
    let page_kib = (unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).max(4096) as u64 / 1024;
    let statm = std::fs::read_to_string(dir.join("statm")).ok()?;
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(pages * page_kib)
}

#[cfg(not(target_os = "linux"))]
fn measure(_pid: u32) -> Option<u64> {
    None
}

#[cfg(target_os = "macos")]
fn list() -> Vec<ProcessMemory> {
    std::process::Command::new("/bin/ps")
        .args(["-A", "-o", "pid=,ppid=,rss=,comm="])
        .output()
        .map(|out| parse_ps(&String::from_utf8_lossy(&out.stdout)))
        .unwrap_or_default()
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn list() -> Vec<ProcessMemory> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, parent: u32) -> ProcessMemory {
        ProcessMemory {
            pid,
            parent,
            name: format!("p{pid}"),
            kib: pid as u64,
        }
    }

    #[test]
    fn keeps_only_the_tree() {
        let all = vec![p(1, 0), p(10, 1), p(11, 10), p(12, 11), p(20, 1), p(13, 10)];
        let pids: Vec<u32> = descendants(10, all).iter().map(|p| p.pid).collect();
        assert_eq!(pids[0], 10);
        let mut rest = pids[1..].to_vec();
        rest.sort();
        assert_eq!(rest, vec![11, 12, 13]);
    }

    #[test]
    fn parses_stat_with_odd_names() {
        let stat = "4242 (Qt (Web) Engine) S 4200 4242 4200 0 -1";
        assert_eq!(parse_stat(stat), Some(("Qt (Web) Engine".into(), 4200)));
        assert_eq!(parse_stat("garbage"), None);
    }

    #[test]
    fn parses_smaps_and_ps() {
        let smaps = "55d0-7ffd ---p 00000000 00:00 0 [rollup]\nRss: 2048 kB\nPss: 1500 kB\n";
        assert_eq!(parse_smaps_rollup(smaps), Some(1500));
        let ps = "  501     1  81234 /Applications/Ion.app/Contents/MacOS/ion\n  502   501 40000 /x/QtWebEngineProcess.app/Contents/MacOS/QtWebEngineProcess\n";
        let rows = parse_ps(ps);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].parent, 501);
        assert_eq!(rows[1].name, "QtWebEngineProcess");
        assert_eq!(rows[0].kib, 81234);
    }

    #[test]
    fn measures_this_process() {
        if cfg!(any(target_os = "linux", target_os = "macos")) {
            let me = tree(std::process::id());
            assert_eq!(me.first().map(|p| p.pid), Some(std::process::id()));
            assert!(me[0].kib > 0);
        }
    }
}
