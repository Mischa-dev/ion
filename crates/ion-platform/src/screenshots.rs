//! Where screenshots of pages are saved.
//!
//! `$XDG_PICTURES_DIR` (from `user-dirs.dirs` when the variable isn't set),
//! else `~/Pictures`, plus an `Ion` folder; on macOS `~/Pictures/Ion`. Files
//! are named after the page and the time, so they sort by when they were
//! taken.

use std::path::{Path, PathBuf};

/// The folder screenshots go to, from `home` and the XDG settings.
pub fn folder(home: &Path) -> PathBuf {
    let pictures = if cfg!(target_os = "macos") {
        None
    } else {
        std::env::var_os("XDG_PICTURES_DIR")
            .map(PathBuf::from)
            .or_else(|| user_dirs_pictures(home))
    };
    pictures
        .unwrap_or_else(|| home.join("Pictures"))
        .join("Ion")
}

/// `XDG_PICTURES_DIR` from `~/.config/user-dirs.dirs`, the file xdg-user-dirs
/// writes (`XDG_PICTURES_DIR="$HOME/Pictures"`).
fn user_dirs_pictures(home: &Path) -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let text = std::fs::read_to_string(config.join("user-dirs.dirs")).ok()?;
    parse_user_dirs(&text, home)
}

fn parse_user_dirs(text: &str, home: &Path) -> Option<PathBuf> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("XDG_PICTURES_DIR="))?;
    let value = line["XDG_PICTURES_DIR=".len()..].trim_matches('"');
    let path = match value.strip_prefix("$HOME") {
        Some(rest) => home.join(rest.trim_start_matches('/')),
        None => PathBuf::from(value),
    };
    // A pictures dir equal to home means "not set" to xdg-user-dirs.
    (path.is_absolute() && path != home).then_some(path)
}

/// A file name for a screenshot of a page titled `title`, taken at `stamp`
/// ("2026-10-07 11-42-03"): "Example Domain 2026-10-07 11-42-03.png".
pub fn file_name(title: &str, stamp: &str) -> String {
    let clean: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    let clean: String = clean.chars().take(80).collect();
    let clean = clean.trim().trim_start_matches('.');
    if clean.is_empty() {
        format!("Screenshot {stamp}.png")
    } else {
        format!("{clean} {stamp}.png")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_safe_and_bounded() {
        assert_eq!(
            file_name("a/b: c?  d", "2026-10-07 11-00-00"),
            "a b c d 2026-10-07 11-00-00.png"
        );
        assert_eq!(file_name("  ", "S"), "Screenshot S.png");
        assert_eq!(file_name("..hidden", "S"), "hidden S.png");
        assert!(file_name(&"x".repeat(500), "S").len() < 100);
    }

    #[test]
    fn user_dirs_file_is_understood() {
        let home = Path::new("/home/me");
        assert_eq!(
            parse_user_dirs("# c\nXDG_PICTURES_DIR=\"$HOME/Bilder\"\n", home),
            Some(PathBuf::from("/home/me/Bilder"))
        );
        assert_eq!(
            parse_user_dirs("XDG_PICTURES_DIR=\"/data/pics\"", home),
            Some(PathBuf::from("/data/pics"))
        );
        assert_eq!(parse_user_dirs("XDG_PICTURES_DIR=\"$HOME/\"", home), None);
        assert_eq!(parse_user_dirs("", home), None);
    }
}
