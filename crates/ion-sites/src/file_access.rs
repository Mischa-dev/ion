//! Sites asking to read or edit files on this computer (the File System
//! Access API: vscode.dev "Open Folder", Photopea saving in place). The
//! person picked the file or folder in the file picker already; Ion still
//! asks before the site may keep using it, saying what it gets.

/// The question for a page at `origin` wanting `path` (a `file://` URL or a
/// plain path). `directory` when it is a folder, `write` when the site wants
/// to change it, not only read it.
pub fn prompt(origin: &str, path: &str, directory: bool, write: bool) -> String {
    let site = crate::rules::host_of(origin)
        .map(|h| h.strip_prefix("www.").map(str::to_owned).unwrap_or(h))
        .unwrap_or_else(|| "This page".to_owned());
    let path = url::Url::parse(path)
        .ok()
        .filter(|u| u.scheme() == "file")
        .and_then(|u| u.to_file_path().ok())
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| path.to_owned());
    let name = home_relative(&path);
    match (directory, write) {
        (true, true) => format!("Let {site} see and change the files in {name}?"),
        (true, false) => format!("Let {site} see the files in {name}?"),
        (false, true) => format!("Let {site} change {name}?"),
        (false, false) => format!("Let {site} read {name}?"),
    }
}

/// `path` with the home folder shown as `~`.
fn home_relative(path: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    match path.strip_prefix(&home) {
        Some(rest) if !home.is_empty() && (rest.is_empty() || rest.starts_with('/')) => {
            format!("~{rest}")
        }
        _ => path.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn says_what_the_site_gets() {
        assert_eq!(
            prompt("https://vscode.dev", "file:///srv/project", true, true),
            "Let vscode.dev see and change the files in /srv/project?"
        );
        assert_eq!(
            prompt("https://www.photopea.com/", "/srv/a b.psd", false, true),
            "Let photopea.com change /srv/a b.psd?"
        );
        assert_eq!(
            prompt("https://x.org", "file:///srv/notes.txt", false, false),
            "Let x.org read /srv/notes.txt?"
        );
    }

    #[test]
    fn home_is_shortened() {
        let home = std::env::var("HOME").unwrap_or_default();
        if home.len() > 1 {
            assert_eq!(home_relative(&format!("{home}/code")), "~/code");
            assert_eq!(
                home_relative(&format!("{home}x/code")),
                format!("{home}x/code")
            );
        }
    }
}
