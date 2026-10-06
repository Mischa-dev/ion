//! Downloads: where a file goes (per-type folders), a free file name there,
//! and the progress line shown in the downloads panel.

use std::collections::BTreeMap;
use std::path::Path;

/// Broad file kinds that can each have their own download folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Category {
    Document,
    Image,
    Audio,
    Video,
    Archive,
    Other,
}

impl Category {
    pub const ALL: [Category; 6] = [
        Category::Document,
        Category::Image,
        Category::Audio,
        Category::Video,
        Category::Archive,
        Category::Other,
    ];

    /// Stable lowercase name, used in config and by QML.
    pub fn name(self) -> &'static str {
        match self {
            Category::Document => "document",
            Category::Image => "image",
            Category::Audio => "audio",
            Category::Video => "video",
            Category::Archive => "archive",
            Category::Other => "other",
        }
    }

    pub fn from_name(name: &str) -> Option<Category> {
        Category::ALL
            .into_iter()
            .find(|c| c.name().eq_ignore_ascii_case(name.trim()))
    }

    /// Classify a download by its MIME type, falling back to the file
    /// extension when the server sent a generic type.
    pub fn classify(file_name: &str, mime_type: &str) -> Category {
        let mime = mime_type
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let by_mime = match mime.split_once('/') {
            Some(("image", _)) => Some(Category::Image),
            Some(("audio", _)) => Some(Category::Audio),
            Some(("video", _)) => Some(Category::Video),
            Some(("text", _)) => Some(Category::Document),
            _ => match mime.as_str() {
                "application/pdf"
                | "application/msword"
                | "application/rtf"
                | "application/epub+zip" => Some(Category::Document),
                m if m.starts_with("application/vnd.openxmlformats-officedocument")
                    || m.starts_with("application/vnd.oasis.opendocument") =>
                {
                    Some(Category::Document)
                }
                "application/zip"
                | "application/gzip"
                | "application/x-tar"
                | "application/x-xz"
                | "application/x-bzip2"
                | "application/x-7z-compressed"
                | "application/vnd.rar"
                | "application/zstd" => Some(Category::Archive),
                _ => None,
            },
        };
        by_mime.unwrap_or_else(|| Self::from_extension(file_name))
    }

    fn from_extension(file_name: &str) -> Category {
        let lower = file_name.to_ascii_lowercase();
        // Compound archive extensions first.
        if [".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst"]
            .iter()
            .any(|ext| lower.ends_with(ext))
        {
            return Category::Archive;
        }
        let ext = match lower.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() => ext,
            _ => return Category::Other,
        };
        match ext {
            "pdf" | "doc" | "docx" | "odt" | "rtf" | "txt" | "md" | "epub" | "xls" | "xlsx"
            | "ods" | "ppt" | "pptx" | "odp" | "csv" => Category::Document,
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "avif" | "svg" | "bmp" | "heic" | "tiff" => {
                Category::Image
            }
            "mp3" | "flac" | "ogg" | "opus" | "wav" | "m4a" | "aac" => Category::Audio,
            "mp4" | "mkv" | "webm" | "mov" | "avi" | "m4v" => Category::Video,
            "zip" | "gz" | "tgz" | "xz" | "bz2" | "7z" | "rar" | "tar" | "zst" => Category::Archive,
            _ => Category::Other,
        }
    }
}

/// Per-type download folders. Kinds without a folder go to the default
/// download directory, so with no rules everything lands in one place.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FolderRules {
    folders: BTreeMap<Category, String>,
}

impl FolderRules {
    pub fn new() -> Self {
        Self::default()
    }

    /// Send `category` downloads to `folder`; an empty folder clears the rule.
    pub fn set(&mut self, category: Category, folder: &str) {
        let folder = folder.trim();
        if folder.is_empty() {
            self.folders.remove(&category);
        } else {
            self.folders.insert(category, folder.to_owned());
        }
    }

    /// The folder for a download, or `default_dir` when no rule matches.
    pub fn folder_for<'a>(
        &'a self,
        file_name: &str,
        mime_type: &str,
        default_dir: &'a str,
    ) -> &'a str {
        self.folders
            .get(&Category::classify(file_name, mime_type))
            .map(String::as_str)
            .unwrap_or(default_dir)
    }
}

/// Make a suggested file name safe to create: no path separators, control
/// characters or leading dots, never empty.
pub fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.').trim();
    if cleaned.is_empty() {
        "download".to_owned()
    } else {
        cleaned.to_owned()
    }
}

/// A file name in `dir` that does not exist yet, numbering the stem the way
/// file managers do: `report.pdf`, `report (1).pdf`, `report (2).pdf`.
pub fn unique_file_name(dir: &Path, name: &str) -> String {
    unique_name_with(name, |candidate| dir.join(candidate).exists())
}

/// [`unique_file_name`] with the existence check supplied by the caller.
pub fn unique_name_with(name: &str, exists: impl Fn(&str) -> bool) -> String {
    let name = sanitize_file_name(name);
    if !exists(&name) {
        return name;
    }
    let (stem, ext) = split_extension(&name);
    (1..)
        .map(|n| format!("{stem} ({n}){ext}"))
        .find(|candidate| !exists(candidate))
        .expect("an unbounded range always yields a free name")
}

/// Split `name` into stem and extension (with its dot), keeping compound
/// archive extensions like `.tar.gz` together.
fn split_extension(name: &str) -> (&str, &str) {
    let lower = name.to_ascii_lowercase();
    for compound in [".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst"] {
        if lower.ends_with(compound) && name.len() > compound.len() {
            let at = name.len() - compound.len();
            return (&name[..at], &name[at..]);
        }
    }
    match name.rfind('.') {
        Some(at) if at > 0 => (&name[..at], &name[at..]),
        _ => (name, ""),
    }
}

/// A byte count for people: `512 B`, `1.4 KB`, `23 MB`, `1.2 GB`.
/// Uses decimal units like GNOME and macOS do.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 999.95 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if value < 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.0} {}", UNITS[unit])
    }
}

/// A duration for a "time left" label: `8 s`, `3 min`, `1 h 20 min`.
pub fn format_duration(seconds: u64) -> String {
    match seconds {
        0..=59 => format!("{seconds} s"),
        60..=3599 => format!("{} min", seconds.div_ceil(60)),
        _ => {
            let minutes = seconds.div_ceil(60);
            let (h, m) = (minutes / 60, minutes % 60);
            if m == 0 {
                format!("{h} h")
            } else {
                format!("{h} h {m} min")
            }
        }
    }
}

/// Where a download is in its life, mirroring `WebEngineDownloadRequest.state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Requested,
    InProgress,
    Completed,
    Cancelled,
    Interrupted,
}

impl State {
    /// From `WebEngineDownloadRequest::DownloadState`'s integer value.
    pub fn from_qt(value: i32) -> State {
        match value {
            1 => State::InProgress,
            2 => State::Completed,
            3 => State::Cancelled,
            4 => State::Interrupted,
            _ => State::Requested,
        }
    }
}

/// A snapshot of one download, enough to describe it.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pub state: State,
    pub paused: bool,
    pub received: u64,
    /// Total size, `None` when the server did not say.
    pub total: Option<u64>,
    /// Smoothed transfer rate in bytes per second.
    pub bytes_per_second: f64,
    /// Why an interrupted download stopped, as reported by the engine.
    pub interrupt_reason: String,
}

impl Progress {
    /// Fraction done in `0.0..=1.0`, or `None` when the total is unknown.
    pub fn fraction(&self) -> Option<f64> {
        match (self.state, self.total) {
            (State::Completed, _) => Some(1.0),
            (_, Some(total)) if total > 0 => Some((self.received as f64 / total as f64).min(1.0)),
            _ => None,
        }
    }

    /// The one-line status under the file name in the downloads panel.
    pub fn status_text(&self) -> String {
        let received = format_bytes(self.received);
        let sizes = match self.total {
            Some(total) if total > 0 => format!("{received} of {}", format_bytes(total)),
            _ => received,
        };
        match self.state {
            State::Requested => "Starting…".to_owned(),
            State::Completed => self
                .total
                .filter(|t| *t > 0)
                .map(format_bytes)
                .unwrap_or_else(|| format_bytes(self.received)),
            State::Cancelled => "Cancelled".to_owned(),
            State::Interrupted if self.interrupt_reason.is_empty() => "Failed".to_owned(),
            State::Interrupted => format!("Failed: {}", self.interrupt_reason),
            State::InProgress if self.paused => format!("Paused · {sizes}"),
            State::InProgress => {
                let rate = self.bytes_per_second;
                let left = self
                    .total
                    .filter(|total| *total > self.received && rate >= 1.0)
                    .map(|total| ((total - self.received) as f64 / rate).ceil() as u64);
                match left {
                    Some(seconds) => format!("{sizes} · {} left", format_duration(seconds)),
                    None if rate >= 1.0 => {
                        format!("{sizes} · {}/s", format_bytes(rate as u64))
                    }
                    None => sizes,
                }
            }
        }
    }
}

/// Smooth a transfer rate sample so the "time left" label does not jump
/// around. `previous` of zero means there is no history yet.
pub fn smooth_rate(previous: f64, sample: f64) -> f64 {
    const WEIGHT: f64 = 0.3;
    let sample = sample.max(0.0);
    if previous <= 0.0 {
        sample
    } else {
        previous + WEIGHT * (sample - previous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn classify_prefers_mime_type() {
        assert_eq!(Category::classify("x.bin", "image/png"), Category::Image);
        assert_eq!(
            Category::classify("a", "application/pdf; charset=binary"),
            Category::Document
        );
        assert_eq!(
            Category::classify(
                "a",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            ),
            Category::Document
        );
        assert_eq!(
            Category::classify("x", "application/zip"),
            Category::Archive
        );
    }

    #[test]
    fn classify_falls_back_to_extension() {
        let generic = "application/octet-stream";
        assert_eq!(Category::classify("song.FLAC", generic), Category::Audio);
        assert_eq!(Category::classify("clip.mkv", generic), Category::Video);
        assert_eq!(Category::classify("src.tar.gz", ""), Category::Archive);
        assert_eq!(Category::classify("setup.exe", generic), Category::Other);
        assert_eq!(Category::classify(".bashrc", generic), Category::Other);
        assert_eq!(Category::classify("README", generic), Category::Other);
    }

    #[test]
    fn category_names_round_trip() {
        for category in Category::ALL {
            assert_eq!(Category::from_name(category.name()), Some(category));
        }
        assert_eq!(Category::from_name(" Image "), Some(Category::Image));
        assert_eq!(Category::from_name("nope"), None);
    }

    #[test]
    fn folder_rules_default_to_the_download_dir() {
        let mut rules = FolderRules::new();
        assert_eq!(rules.folder_for("a.png", "image/png", "/dl"), "/dl");
        rules.set(Category::Image, "/pics");
        assert_eq!(rules.folder_for("a.png", "image/png", "/dl"), "/pics");
        assert_eq!(rules.folder_for("a.pdf", "application/pdf", "/dl"), "/dl");
        rules.set(Category::Image, "  ");
        assert_eq!(rules.folder_for("a.png", "image/png", "/dl"), "/dl");
    }

    #[test]
    fn sanitize_removes_separators_and_dots() {
        assert_eq!(sanitize_file_name("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(sanitize_file_name("a\\b:c"), "a_b_c");
        assert_eq!(sanitize_file_name("..hidden"), "hidden");
        assert_eq!(sanitize_file_name("  "), "download");
        assert_eq!(sanitize_file_name("tab\there"), "tab_here");
        assert_eq!(sanitize_file_name("résumé.pdf"), "résumé.pdf");
    }

    #[test]
    fn unique_names_number_the_stem() {
        let taken: HashSet<&str> = ["report.pdf", "report (1).pdf", "src.tar.gz", "README"]
            .into_iter()
            .collect();
        let exists = |n: &str| taken.contains(n);
        assert_eq!(unique_name_with("new.pdf", exists), "new.pdf");
        assert_eq!(unique_name_with("report.pdf", exists), "report (2).pdf");
        assert_eq!(unique_name_with("src.tar.gz", exists), "src (1).tar.gz");
        assert_eq!(unique_name_with("README", exists), "README (1)");
    }

    #[test]
    fn unique_file_name_checks_the_directory() {
        let dir = std::env::temp_dir().join(format!("ion-basics-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "x").unwrap();
        assert_eq!(unique_file_name(&dir, "a.txt"), "a (1).txt");
        assert_eq!(unique_file_name(&dir, "b.txt"), "b.txt");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn bytes_are_human_readable() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(1_000), "1.0 KB");
        assert_eq!(format_bytes(1_450), "1.4 KB");
        assert_eq!(format_bytes(23_400_000), "23 MB");
        assert_eq!(format_bytes(999_999), "1.0 MB");
        assert_eq!(format_bytes(1_200_000_000), "1.2 GB");
    }

    #[test]
    fn durations_are_human_readable() {
        assert_eq!(format_duration(8), "8 s");
        assert_eq!(format_duration(61), "2 min");
        assert_eq!(format_duration(3600), "1 h");
        assert_eq!(format_duration(4800), "1 h 20 min");
    }

    fn progress(state: State) -> Progress {
        Progress {
            state,
            paused: false,
            received: 5_000_000,
            total: Some(10_000_000),
            bytes_per_second: 1_000_000.0,
            interrupt_reason: String::new(),
        }
    }

    #[test]
    fn status_text_in_progress() {
        assert_eq!(
            progress(State::InProgress).status_text(),
            "5.0 MB of 10 MB · 5 s left"
        );
        let unknown_total = Progress {
            total: None,
            ..progress(State::InProgress)
        };
        assert_eq!(unknown_total.status_text(), "5.0 MB · 1.0 MB/s");
        let stalled = Progress {
            bytes_per_second: 0.0,
            ..progress(State::InProgress)
        };
        assert_eq!(stalled.status_text(), "5.0 MB of 10 MB");
        let paused = Progress {
            paused: true,
            ..progress(State::InProgress)
        };
        assert_eq!(paused.status_text(), "Paused · 5.0 MB of 10 MB");
    }

    #[test]
    fn status_text_when_finished() {
        assert_eq!(progress(State::Completed).status_text(), "10 MB");
        assert_eq!(progress(State::Cancelled).status_text(), "Cancelled");
        assert_eq!(progress(State::Interrupted).status_text(), "Failed");
        let failed = Progress {
            interrupt_reason: "Network disconnected".into(),
            ..progress(State::Interrupted)
        };
        assert_eq!(failed.status_text(), "Failed: Network disconnected");
        assert_eq!(progress(State::Requested).status_text(), "Starting…");
    }

    #[test]
    fn fraction() {
        assert_eq!(progress(State::InProgress).fraction(), Some(0.5));
        assert_eq!(progress(State::Completed).fraction(), Some(1.0));
        let unknown = Progress {
            total: None,
            ..progress(State::InProgress)
        };
        assert_eq!(unknown.fraction(), None);
    }

    #[test]
    fn state_from_qt() {
        assert_eq!(State::from_qt(0), State::Requested);
        assert_eq!(State::from_qt(1), State::InProgress);
        assert_eq!(State::from_qt(2), State::Completed);
        assert_eq!(State::from_qt(3), State::Cancelled);
        assert_eq!(State::from_qt(4), State::Interrupted);
    }

    #[test]
    fn rate_smoothing() {
        assert_eq!(smooth_rate(0.0, 100.0), 100.0);
        assert_eq!(smooth_rate(100.0, 200.0), 130.0);
        assert_eq!(smooth_rate(100.0, -5.0), 70.0);
    }
}
