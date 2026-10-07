//! Ion commands the palette can run.
//!
//! The ids are the contract with QML: `CommandPalette.qml` maps each id to
//! what it does. Adding a command means one entry here and one case there.

/// One palette command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    /// Stable id passed to QML.
    pub id: &'static str,
    /// Human-facing name.
    pub title: &'static str,
    /// Extra words people might search for.
    pub keywords: &'static str,
    /// Shortcut hint in portable form ("Ctrl" is shown as ⌘ on macOS).
    pub shortcut: &'static str,
}

const fn command(
    id: &'static str,
    title: &'static str,
    keywords: &'static str,
    shortcut: &'static str,
) -> Command {
    Command {
        id,
        title,
        keywords,
        shortcut,
    }
}

/// Every command, in the order shown for an empty query.
pub const COMMANDS: &[Command] = &[
    command("new-tab", "New tab", "open create", "Ctrl+T"),
    command("close-tab", "Close tab", "remove", "Ctrl+W"),
    command("duplicate-tab", "Duplicate tab", "copy clone", ""),
    command(
        "reopen-tab",
        "Reopen closed tab",
        "undo restore",
        "Ctrl+Shift+T",
    ),
    command("next-tab", "Next tab", "switch right", "Ctrl+Tab"),
    command(
        "previous-tab",
        "Previous tab",
        "switch left",
        "Ctrl+Shift+Tab",
    ),
    command("focus-url", "Edit address", "url location bar", "Ctrl+L"),
    command("reload", "Reload page", "refresh", "Ctrl+R"),
    command("back", "Go back", "history previous", "Alt+Left"),
    command("forward", "Go forward", "history next", "Alt+Right"),
    command(
        "bookmark-page",
        "Bookmark this page",
        "star favorite save remove unbookmark",
        "Ctrl+D",
    ),
    command("bookmarks", "Show bookmarks", "favorites starred list", ""),
    command(
        "import-browser-data",
        "Import bookmarks and history",
        "chrome firefox brave chromium vivaldi edge migrate",
        "",
    ),
    command(
        "clear-cookies",
        "Clear all cookies (signs you out of sites)",
        "privacy delete sign out logout site data",
        "",
    ),
    command("clear-cache", "Clear cache", "privacy delete http disk", ""),
    command(
        "clear-history",
        "Clear all history",
        "privacy delete forget visited pages",
        "",
    ),
    command(
        "screenshot",
        "Take a screenshot of the page",
        "capture image png picture copy",
        "Ctrl+Shift+S",
    ),
    command(
        "reader-mode",
        "Reader mode",
        "read article clean distraction free simplify",
        "Ctrl+Alt+R",
    ),
    command(
        "reload-userscripts",
        "Reload user scripts and styles",
        "userscript css greasemonkey refresh",
        "",
    ),
    command(
        "open-userscripts",
        "Open user scripts folder",
        "userscript css greasemonkey edit",
        "",
    ),
    command("quit", "Quit Ion", "exit close window", "Ctrl+Q"),
];

/// Look up a command by id.
pub fn find(id: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        for (i, a) in COMMANDS.iter().enumerate() {
            assert!(
                COMMANDS[i + 1..].iter().all(|b| a.id != b.id),
                "duplicate id {}",
                a.id
            );
        }
    }

    #[test]
    fn lookup_by_id() {
        assert_eq!(find("reload").map(|c| c.title), Some("Reload page"));
        assert_eq!(find("nope"), None);
    }
}
