//! Ion commands: what the palette can run and what shortcuts are bound to.
//!
//! The ids are the contract with QML: `Commands.qml` maps each id to what it
//! does. Adding a command means one entry here and one case there. Default
//! keys are Qt's portable key text, where `Ctrl` is Cmd on macOS; people
//! remap them in `[shortcuts]` or the shortcut editor (`ion_input::keymap`).

/// One command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    /// Stable id passed to QML.
    pub id: &'static str,
    /// Human-facing name.
    pub title: &'static str,
    /// Extra words people might search for.
    pub keywords: &'static str,
    /// Default keys, the first shown as the hint.
    pub keys: &'static [&'static str],
    /// Only a shortcut: left out of the palette's list (Ctrl+1…9, the palette
    /// itself), still listed in the shortcut editor.
    pub hidden: bool,
}

const fn command(
    id: &'static str,
    title: &'static str,
    keywords: &'static str,
    keys: &'static [&'static str],
) -> Command {
    Command {
        id,
        title,
        keywords,
        keys,
        hidden: false,
    }
}

/// Commands that only make sense with Ion Agent turned on (`ai.enable`).
pub const AGENT_COMMANDS: &[&str] = &[
    "ask-agent",
    "summarize-page",
    "agent-sidebar",
    "agent-activity",
];

const fn shortcut(id: &'static str, title: &'static str, keys: &'static [&'static str]) -> Command {
    Command {
        id,
        title,
        keywords: "",
        keys,
        hidden: true,
    }
}

const MAC: bool = cfg!(target_os = "macos");

// Platform conventions Qt's StandardKey would pick.
const BACK: &[&str] = if MAC {
    &["Ctrl+[", "Ctrl+Left"]
} else {
    &["Alt+Left", "Back"]
};
const FORWARD: &[&str] = if MAC {
    &["Ctrl+]", "Ctrl+Right"]
} else {
    &["Alt+Right", "Forward"]
};
// Cmd+Tab belongs to macOS, so Control+Tab (Meta in Qt's terms) there.
const NEXT_TAB: &[&str] = if MAC {
    &["Meta+Tab", "Ctrl+}", "Ctrl+Alt+Right", "Ctrl+PgDown"]
} else {
    &["Ctrl+Tab", "Ctrl+PgDown"]
};
const PREVIOUS_TAB: &[&str] = if MAC {
    &["Meta+Shift+Tab", "Ctrl+{", "Ctrl+Alt+Left", "Ctrl+PgUp"]
} else {
    &["Ctrl+Shift+Tab", "Ctrl+PgUp"]
};
const CLOSE_TAB: &[&str] = if MAC {
    &["Ctrl+W"]
} else {
    &["Ctrl+W", "Ctrl+F4"]
};
const FULL_SCREEN: &[&str] = if MAC { &["Ctrl+Meta+F"] } else { &["F11"] };
const FIND_NEXT: &[&str] = if MAC { &["Ctrl+G"] } else { &["F3", "Ctrl+G"] };
const FIND_PREVIOUS: &[&str] = if MAC {
    &["Ctrl+Shift+G"]
} else {
    &["Shift+F3", "Ctrl+Shift+G"]
};

/// Every command, in the order shown for an empty query.
pub const COMMANDS: &[Command] = &[
    command("new-tab", "New tab", "open create", &["Ctrl+T"]),
    command("close-tab", "Close tab", "remove", CLOSE_TAB),
    command("duplicate-tab", "Duplicate tab", "copy clone", &[]),
    command(
        "reopen-tab",
        "Reopen closed tab",
        "undo restore",
        &["Ctrl+Shift+T"],
    ),
    command(
        "search-tabs",
        "Search tabs",
        "find switch recently closed",
        &["Ctrl+Shift+A"],
    ),
    command(
        "split-view",
        "Split view",
        "side by side tile two tabs",
        &["Ctrl+Alt+S"],
    ),
    command(
        "sleep-tabs",
        "Put background tabs to sleep",
        "suspend unload discard free memory",
        &[],
    ),
    command("next-tab", "Next tab", "switch right", NEXT_TAB),
    command("previous-tab", "Previous tab", "switch left", PREVIOUS_TAB),
    command(
        "move-tab-left",
        "Move tab left",
        "reorder",
        &["Ctrl+Shift+PgUp"],
    ),
    command(
        "move-tab-right",
        "Move tab right",
        "reorder",
        &["Ctrl+Shift+PgDown"],
    ),
    command(
        "next-workspace",
        "Next workspace",
        "switch space",
        &["Ctrl+Alt+PgDown"],
    ),
    command(
        "previous-workspace",
        "Previous workspace",
        "switch space",
        &["Ctrl+Alt+PgUp"],
    ),
    command("close-other-tabs", "Close other tabs", "remove all", &[]),
    command(
        "focus-url",
        "Edit address",
        "url location bar",
        &["Ctrl+L", "Alt+D", "F6"],
    ),
    command(
        "copy-url",
        "Copy page address",
        "url link clipboard share",
        &[],
    ),
    command("reload", "Reload page", "refresh", &["Ctrl+R", "F5"]),
    command(
        "hard-reload",
        "Reload without cache",
        "refresh force bypass",
        &[],
    ),
    command("stop", "Stop loading", "cancel", &[]),
    command("find", "Find in page", "search text", &["Ctrl+F"]),
    command("find-next", "Find next match", "search", FIND_NEXT),
    command(
        "find-previous",
        "Find previous match",
        "search",
        FIND_PREVIOUS,
    ),
    command(
        "zoom-in",
        "Zoom in",
        "bigger larger magnify",
        &["Ctrl++", "Ctrl+="],
    ),
    command("zoom-out", "Zoom out", "smaller", &["Ctrl+-"]),
    command("zoom-reset", "Reset zoom", "actual size 100%", &["Ctrl+0"]),
    command("back", "Go back", "history previous", BACK),
    command("forward", "Go forward", "history next", FORWARD),
    command(
        "fullscreen",
        "Toggle full screen",
        "maximize window",
        FULL_SCREEN,
    ),
    command(
        "toggle-adblock",
        "Toggle blocking on this site",
        "adblock ads trackers allow pause",
        &[],
    ),
    command(
        "update-filter-lists",
        "Update filter lists",
        "adblock ads trackers download refresh",
        &[],
    ),
    command(
        "bookmark-page",
        "Bookmark this page",
        "star favorite save remove unbookmark",
        &["Ctrl+D"],
    ),
    command(
        "bookmarks",
        "Bookmarks",
        "favorites starred list manage folders",
        &["Ctrl+Shift+O"],
    ),
    command(
        "history",
        "History",
        "visited pages recent past",
        &["Ctrl+H"],
    ),
    command(
        "all-downloads",
        "All downloads",
        "files saved list page",
        &["Ctrl+Shift+J"],
    ),
    command(
        "import-browser-data",
        "Import bookmarks and history",
        "chrome firefox brave chromium vivaldi edge migrate",
        &[],
    ),
    command(
        "new-private-window",
        "New private window",
        "incognito secret anonymous history off the record",
        &["Ctrl+Shift+N"],
    ),
    command(
        "profiles",
        "Switch profile",
        "profiles accounts work personal separate cookies sign in containers",
        &[],
    ),
    command(
        "passwords",
        "Passwords",
        "logins keys keychain saved accounts manager vault credentials",
        &[],
    ),
    command(
        "addresses",
        "Addresses",
        "autofill shipping billing home contact forms saved",
        &[],
    ),
    command(
        "generate-password",
        "Copy a strong new password",
        "generate random secure create",
        &[],
    ),
    command(
        "clear-cookies",
        "Clear all cookies (signs you out of sites)",
        "privacy delete sign out logout site data",
        &[],
    ),
    command(
        "clear-cache",
        "Clear cache",
        "privacy delete http disk",
        &[],
    ),
    command(
        "clear-history",
        "Clear all history",
        "privacy delete forget visited pages",
        &[],
    ),
    command(
        "extensions",
        "Extensions",
        "addons plugins chrome manage enable disable",
        &[],
    ),
    command(
        "open-extensions-folder",
        "Open extensions folder",
        "addons plugins chrome install unpacked",
        &[],
    ),
    command(
        "reader-mode",
        "Reader mode",
        "read article clean distraction free simplify",
        &["Ctrl+Alt+R"],
    ),
    command(
        "reload-userscripts",
        "Reload user scripts and styles",
        "userscript css greasemonkey refresh",
        &[],
    ),
    command(
        "open-userscripts",
        "Open user scripts folder",
        "userscript css greasemonkey edit",
        &[],
    ),
    command(
        "stop-agents",
        "Stop all agents",
        "ai halt pause safety",
        &["Ctrl+Shift+."],
    ),
    command("resume-agents", "Resume agents", "ai start safety", &[]),
    command(
        "ask-agent",
        "Ask Ion Agent",
        "ai assistant chat question @ion",
        &["Ctrl+E"],
    ),
    command(
        "summarize-page",
        "Summarize page with Ion Agent",
        "ai assistant tldr summary explain",
        &[],
    ),
    command(
        "set-up-agent",
        "Set up Ion Agent",
        "ai assistant model api key claude openai ollama provider",
        &[],
    ),
    command(
        "agent-activity",
        "Agent activity",
        "ai log history permissions safety allowed take back",
        &[],
    ),
    command(
        "agent-sidebar",
        "Show or hide agent sidebar",
        "ai assistant chat conversation panel",
        &["Ctrl+Shift+E"],
    ),
    command(
        "screenshot",
        "Take screenshot",
        "capture image copy visible",
        &["Ctrl+Shift+S"],
    ),
    command(
        "screenshot-page",
        "Screenshot whole page",
        "capture image copy entire long scrolling",
        &["Ctrl+Alt+Shift+S"],
    ),
    command("downloads", "Show downloads", "files saved", &["Ctrl+J"]),
    command(
        "keyboard-shortcuts",
        "Keyboard shortcuts",
        "keys keybindings hotkeys remap customize edit",
        &["Ctrl+/"],
    ),
    command(
        "settings",
        "Settings",
        "preferences options config configure",
        &["Ctrl+,"],
    ),
    command(
        "welcome",
        "Set up Ion (welcome page)",
        "onboarding first run tour import theme",
        &[],
    ),
    command(
        "clear-browsing-data",
        "Clear browsing data…",
        "privacy delete history cookies cache forget",
        &[],
    ),
    command("quit", "Quit Ion", "exit close window", &["Ctrl+Q"]),
    shortcut("palette", "Command palette", &["Ctrl+K"]),
    shortcut("tab-1", "First tab", &["Ctrl+1"]),
    shortcut("tab-2", "Second tab", &["Ctrl+2"]),
    shortcut("tab-3", "Third tab", &["Ctrl+3"]),
    shortcut("tab-4", "Fourth tab", &["Ctrl+4"]),
    shortcut("tab-5", "Fifth tab", &["Ctrl+5"]),
    shortcut("tab-6", "Sixth tab", &["Ctrl+6"]),
    shortcut("tab-7", "Seventh tab", &["Ctrl+7"]),
    shortcut("tab-8", "Eighth tab", &["Ctrl+8"]),
    shortcut("last-tab", "Last tab", &["Ctrl+9"]),
];

/// How many recently run commands the palette remembers.
pub const RECENT: usize = 8;

/// Put `id` first in `recent` (most recent first), once, keeping at most
/// [`RECENT`] known commands.
pub fn remember(recent: &mut Vec<String>, id: &str) {
    if find(id).is_none() {
        return;
    }
    recent.retain(|r| r != id && find(r).is_some());
    recent.insert(0, id.to_owned());
    recent.truncate(RECENT);
}

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
    fn no_two_commands_share_a_default_key() {
        let keys: Vec<&str> = COMMANDS
            .iter()
            .flat_map(|c| c.keys.iter().copied())
            .collect();
        for (i, key) in keys.iter().enumerate() {
            assert!(!keys[i + 1..].contains(key), "{key} is bound twice");
        }
    }

    #[test]
    fn remember_keeps_recent_first_and_unique() {
        let mut recent = vec!["reload".to_string(), "gone".to_string()];
        remember(&mut recent, "new-tab");
        remember(&mut recent, "reload");
        remember(&mut recent, "not-a-command");
        assert_eq!(recent, ["reload", "new-tab"]);
        for c in COMMANDS {
            remember(&mut recent, c.id);
        }
        assert_eq!(recent.len(), RECENT);
    }

    #[test]
    fn lookup_by_id() {
        assert_eq!(find("reload").map(|c| c.title), Some("Reload page"));
        assert_eq!(find("nope"), None);
    }
}
