//! Private windows and profiles.
//!
//! Each runs as its own Ion process, so nothing can leak between them:
//!
//! - `ion --private`: an off-the-record browser profile (cookies, cache and
//!   site data live in memory and vanish when the window closes), no history,
//!   no saved session. Every private launch is a new window of its own.
//! - `ion --profile=Work`: a separate persistent profile with its own cookies
//!   and sign-ins, history, bookmarks, passwords and sessions, in
//!   `<data dir>/profiles/Work`. The settings file is shared.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Which kind of window this process is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// The default profile.
    Normal,
    /// A private window.
    Private,
    /// A named profile.
    Profile(String),
}

impl Mode {
    /// From the command line: `--private`, or `--profile=NAME`.
    pub fn from_args<I: IntoIterator<Item = S>, S: AsRef<str>>(args: I) -> Self {
        let mut mode = Self::Normal;
        for arg in args {
            let arg = arg.as_ref();
            if arg == "--private" || arg == "--incognito" {
                mode = Self::Private;
            } else if let Some(name) = arg.strip_prefix("--profile=")
                && let Some(name) = clean_name(name)
                && mode == Self::Normal
            {
                mode = Self::Profile(name);
            }
        }
        mode
    }

    /// The QtWebEngine storage name: empty (off the record) for a private
    /// window, `Default` for the default profile.
    pub fn storage_name(&self) -> String {
        match self {
            Self::Normal => "Default".into(),
            Self::Private => String::new(),
            Self::Profile(name) => format!("Profile-{name}"),
        }
    }

    /// The name used for the single-instance socket: one running process
    /// per profile. `None` for private windows, which never hand off.
    pub fn instance_name(&self) -> Option<String> {
        match self {
            Self::Normal => Some("Default".into()),
            Self::Private => None,
            Self::Profile(name) => Some(format!("Profile-{name}")),
        }
    }

    /// Where this profile keeps its data, given the default data folder.
    pub fn data_dir(&self, base: &Path) -> PathBuf {
        match self {
            Self::Profile(name) => base.join("profiles").join(name),
            _ => base.to_path_buf(),
        }
    }

    /// The profile's name for people, empty for the default profile.
    pub fn label(&self) -> &str {
        match self {
            Self::Normal => "",
            Self::Private => "Private",
            Self::Profile(name) => name,
        }
    }

    /// Arguments that start another window in this mode.
    pub fn args(&self) -> Vec<String> {
        match self {
            Self::Normal => Vec::new(),
            Self::Private => vec!["--private".into()],
            Self::Profile(name) => vec![format!("--profile={name}")],
        }
    }
}

/// A profile name safe as a folder name: trimmed, at most 40 characters,
/// letters, digits, spaces, `-` and `_` only.
pub fn clean_name(name: &str) -> Option<String> {
    let name = name.trim();
    let ok = !name.is_empty()
        && name.chars().count() <= 40
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        && !name.eq_ignore_ascii_case("default");
    ok.then(|| name.to_owned())
}

/// The profiles created so far, by name, under the default data folder.
pub fn profiles(base: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(base.join("profiles"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| clean_name(&entry.file_name().to_string_lossy()))
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

static MODE: OnceLock<Mode> = OnceLock::new();

/// Set this process's mode once, at startup.
pub fn set_mode(mode: Mode) {
    let _ = MODE.set(mode);
}

/// This process's mode ([`Mode::Normal`] until set).
pub fn mode() -> &'static Mode {
    MODE.get_or_init(|| Mode::Normal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_come_from_the_command_line() {
        assert_eq!(Mode::from_args(["https://a.com"]), Mode::Normal);
        assert_eq!(
            Mode::from_args(["--private", "https://a.com"]),
            Mode::Private
        );
        assert_eq!(
            Mode::from_args(["--profile=Work"]),
            Mode::Profile("Work".into())
        );
        // Private wins: a private window never opens a profile's data.
        assert_eq!(
            Mode::from_args(["--profile=Work", "--private"]),
            Mode::Private
        );
        for bad in [
            "--profile=",
            "--profile=../x",
            "--profile=a/b",
            "--profile=Default",
        ] {
            assert_eq!(Mode::from_args([bad]), Mode::Normal, "{bad}");
        }
    }

    #[test]
    fn each_mode_keeps_its_own_storage() {
        let base = Path::new("/data");
        assert_eq!(Mode::Normal.storage_name(), "Default");
        assert_eq!(Mode::Private.storage_name(), "");
        let work = Mode::Profile("Work".into());
        assert_eq!(work.storage_name(), "Profile-Work");
        assert_eq!(work.data_dir(base), Path::new("/data/profiles/Work"));
        assert_eq!(Mode::Private.data_dir(base), base);
        assert_eq!(Mode::Private.instance_name(), None);
        assert_eq!(work.args(), ["--profile=Work"]);
    }

    #[test]
    fn profiles_are_listed_from_their_folders() {
        let base = std::env::temp_dir().join(format!("ion-profiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        assert!(profiles(&base).is_empty());
        for name in ["work", "Personal", "bad.name"] {
            std::fs::create_dir_all(base.join("profiles").join(name)).unwrap();
        }
        assert_eq!(profiles(&base), ["Personal", "work"]);
        std::fs::remove_dir_all(base).unwrap();
    }
}
