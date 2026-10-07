//! The ordered list of open tabs.

use crate::session::{SavedTab, SavedWorkspace, Session};

/// Stable identity of a tab for as long as it is open. Indices shift as tabs
/// open, close and move; ids do not.
pub type TabId = u64;

/// Stable identity of a workspace, saved with the session.
pub type WorkspaceId = u32;

/// Name of the workspace every tab list starts with.
pub const DEFAULT_WORKSPACE_NAME: &str = "Default";

/// A named group of tabs with its own color. The strip shows the current
/// workspace's tabs only; the current workspace is the current tab's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    /// A `#rrggbb` color, or empty for the theme's accent.
    pub color: String,
    /// A short label shown instead of the color dot, usually an emoji, or
    /// empty.
    pub icon: String,
}

/// Longest workspace icon kept, in characters: one emoji can take several.
pub const WORKSPACE_ICON_MAX_CHARS: usize = 8;

/// How many closed tabs "reopen closed tab" remembers.
pub const CLOSED_TABS_KEPT: usize = 25;

/// One open tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
    pub id: TabId,
    /// The page's current address (empty for a fresh tab that has not navigated).
    pub url: String,
    /// The page title, or empty until the page reports one.
    pub title: String,
    /// The page's favicon as the web view reports it (an `image://favicon/…`
    /// URL), or empty. Saved, so restored tabs show it before they load.
    pub icon: String,
    /// The page was unloaded to save memory; it reloads when shown again.
    pub suspended: bool,
    /// Unix time (seconds) the tab was last current, 0 until first seen.
    pub last_active: u64,
    pub workspace: WorkspaceId,
}

#[derive(Clone, Debug)]
struct ClosedTab {
    tab: Tab,
    index: usize,
}

/// Open tabs in strip order, which one is current, the workspaces they are
/// grouped into, and recently closed tabs.
///
/// Every mutating method reports what changed so the UI adapter can emit the
/// matching model signals. `current` is always a valid index while the list is
/// not empty, and there is always at least one workspace.
#[derive(Debug)]
pub struct TabList {
    tabs: Vec<Tab>,
    current: usize,
    /// The tab [`note_current`](Self::note_current) last saw as current.
    seen_current: Option<TabId>,
    next_id: TabId,
    closed: Vec<ClosedTab>,
    workspaces: Vec<Workspace>,
    /// The workspace shown while no tab is open.
    empty_workspace: WorkspaceId,
    /// Each workspace's last current tab, to return to when switching back.
    last_current: Vec<(WorkspaceId, TabId)>,
}

impl Default for TabList {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            current: 0,
            seen_current: None,
            next_id: 0,
            closed: Vec::new(),
            workspaces: vec![Workspace {
                id: 0,
                name: DEFAULT_WORKSPACE_NAME.to_owned(),
                color: String::new(),
                icon: String::new(),
            }],
            empty_workspace: 0,
            last_current: Vec::new(),
        }
    }
}

impl TabList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild a tab list from a saved session.
    pub fn from_session(session: &Session) -> Self {
        let mut list = Self::new();
        if !session.workspaces.is_empty() {
            list.workspaces = session
                .workspaces
                .iter()
                .map(|w| Workspace {
                    id: w.id,
                    name: w.name.clone(),
                    color: w.color.clone(),
                    icon: w.icon.clone(),
                })
                .collect();
            list.empty_workspace = list.workspaces[0].id;
        }
        let fallback = list.workspaces[0].id;
        for saved in &session.tabs {
            let index = list.tabs.len();
            list.open(index, &saved.url, &saved.title, false);
            let tab = &mut list.tabs[index];
            tab.icon = saved.icon.clone();
            tab.workspace = if list.workspaces.iter().any(|w| w.id == saved.workspace) {
                saved.workspace
            } else {
                fallback
            };
        }
        list.current = session.current.min(list.tabs.len().saturating_sub(1));
        list
    }

    /// Snapshot the open tabs for saving.
    pub fn to_session(&self) -> Session {
        Session {
            tabs: self
                .tabs
                .iter()
                .map(|t| SavedTab {
                    url: t.url.clone(),
                    title: t.title.clone(),
                    icon: t.icon.clone(),
                    workspace: t.workspace,
                })
                .collect(),
            current: self.current,
            workspaces: self
                .workspaces
                .iter()
                .map(|w| SavedWorkspace {
                    id: w.id,
                    name: w.name.clone(),
                    color: w.color.clone(),
                    icon: w.icon.clone(),
                })
                .collect(),
            ..Session::default()
        }
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn get(&self, index: usize) -> Option<&Tab> {
        self.tabs.get(index)
    }

    /// Index of the current tab, `None` when no tab is open.
    pub fn current(&self) -> Option<usize> {
        (!self.tabs.is_empty()).then_some(self.current)
    }

    pub fn index_of(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    /// Number of tabs that "reopen closed tab" can bring back.
    pub fn closed_count(&self) -> usize {
        self.closed.len()
    }

    /// Open a tab in the current workspace at `index` (clamped to the end) and
    /// return its index.
    pub fn open(&mut self, index: usize, url: &str, title: &str, activate: bool) -> usize {
        let workspace = self.current_workspace();
        self.open_in(workspace, index, url, title, activate)
    }

    /// Open a tab in `workspace` at `index` (clamped to the end) and return its
    /// index.
    pub fn open_in(
        &mut self,
        workspace: WorkspaceId,
        index: usize,
        url: &str,
        title: &str,
        activate: bool,
    ) -> usize {
        let index = index.min(self.tabs.len());
        let tab = Tab {
            id: self.next_id,
            url: url.to_owned(),
            title: title.to_owned(),
            icon: String::new(),
            suspended: false,
            last_active: 0,
            workspace,
        };
        self.next_id += 1;
        self.insert(index, tab, activate)
    }

    fn insert(&mut self, index: usize, tab: Tab, activate: bool) -> usize {
        let was_empty = self.tabs.is_empty();
        self.tabs.insert(index, tab);
        if activate || was_empty {
            self.current = index;
        } else if index <= self.current {
            self.current += 1;
        }
        index
    }

    /// Close the tab at `index`. The current tab stays current; closing the
    /// current tab activates the next tab to its right in the same workspace,
    /// else the nearest one to its left, else the nearest tab at all.
    pub fn close(&mut self, index: usize) -> Option<Tab> {
        if index >= self.tabs.len() {
            return None;
        }
        let tab = self.tabs.remove(index);
        if index < self.current {
            self.current -= 1;
        } else if index == self.current && !self.tabs.is_empty() {
            let same = |t: &Tab| t.workspace == tab.workspace;
            self.current = self.tabs[index..]
                .iter()
                .position(same)
                .map(|i| index + i)
                .or_else(|| self.tabs[..index].iter().rposition(same))
                .unwrap_or(index.min(self.tabs.len() - 1));
        }
        if self.tabs.is_empty() {
            self.current = 0;
            self.empty_workspace = tab.workspace;
        }
        if !tab.url.is_empty() {
            if self.closed.len() == CLOSED_TABS_KEPT {
                self.closed.remove(0);
            }
            self.closed.push(ClosedTab {
                tab: tab.clone(),
                index,
            });
        }
        Some(tab)
    }

    /// Where [`reopen_closed`](Self::reopen_closed) would put the tab back.
    pub fn next_reopen_index(&self) -> Option<usize> {
        self.closed.last().map(|c| c.index.min(self.tabs.len()))
    }

    /// Reopen the most recently closed tab where it was and make it current.
    /// Returns its new index.
    pub fn reopen_closed(&mut self) -> Option<usize> {
        let ClosedTab { mut tab, index } = self.closed.pop()?;
        tab.suspended = false;
        if self.workspace(tab.workspace).is_none() {
            tab.workspace = self.current_workspace();
        }
        let index = index.min(self.tabs.len());
        Some(self.insert(index, tab, true))
    }

    /// Make the tab at `index` current. False if out of range or already current.
    pub fn activate(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() || index == self.current {
            return false;
        }
        self.current = index;
        true
    }

    /// Move the current tab by `step` within its workspace, wrapping around.
    /// Returns the new index.
    pub fn cycle(&mut self, step: isize) -> Option<usize> {
        let rows = self.rows_in(self.current_workspace());
        let at = rows.iter().position(|&i| i == self.current)? as isize;
        self.current = rows[(at + step).rem_euclid(rows.len() as isize) as usize];
        Some(self.current)
    }

    /// Index of the tab `step` places from `index` among the tabs of its
    /// workspace, without wrapping.
    pub fn neighbour(&self, index: usize, step: isize) -> Option<usize> {
        let rows = self.rows_in(self.tabs.get(index)?.workspace);
        let at = rows.iter().position(|&i| i == index)? as isize + step;
        rows.get(usize::try_from(at).ok()?).copied()
    }

    /// Index of the `n`th tab of the current workspace (0-based), or of its
    /// last tab when `n` is negative.
    pub fn nth_in_workspace(&self, n: isize) -> Option<usize> {
        let rows = self.rows_in(self.current_workspace());
        if n < 0 {
            rows.last().copied()
        } else {
            rows.get(n as usize).copied()
        }
    }

    /// Move the tab at `from` so it ends up at `to`. The current tab stays the
    /// same tab. False if either index is out of range or nothing moves.
    pub fn move_tab(&mut self, from: usize, to: usize) -> bool {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return false;
        }
        let current_id = self.tabs[self.current].id;
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        self.current = self.index_of(current_id).unwrap_or(0);
        true
    }

    /// Record the address the tab at `index` now shows. True if it changed.
    pub fn set_url(&mut self, index: usize, url: &str) -> bool {
        match self.tabs.get_mut(index) {
            Some(tab) if tab.url != url => {
                tab.url = url.to_owned();
                true
            }
            _ => false,
        }
    }

    /// Record the title of the tab at `index`. True if it changed.
    pub fn set_title(&mut self, index: usize, title: &str) -> bool {
        match self.tabs.get_mut(index) {
            Some(tab) if tab.title != title => {
                tab.title = title.to_owned();
                true
            }
            _ => false,
        }
    }

    /// Record the favicon of the tab at `index`. True if it changed.
    pub fn set_icon(&mut self, index: usize, icon: &str) -> bool {
        match self.tabs.get_mut(index) {
            Some(tab) if tab.icon != icon => {
                tab.icon = icon.to_owned();
                true
            }
            _ => false,
        }
    }

    /// Record that time `now` has been reached with the current tab showing:
    /// the tab that was current before (if it changed) and the current one are
    /// stamped as active now, tabs never stamped get `now`, and the current tab
    /// is no longer suspended. Returns the current index if that cleared its
    /// suspended flag.
    pub fn note_current(&mut self, now: u64) -> Option<usize> {
        let current = self.current()?;
        let current_id = self.tabs[current].id;
        if let Some(previous) = self.seen_current.filter(|&id| id != current_id) {
            if let Some(i) = self.index_of(previous) {
                self.tabs[i].last_active = now;
            }
        }
        self.seen_current = Some(current_id);
        let workspace = self.tabs[current].workspace;
        match self.last_current.iter_mut().find(|(w, _)| *w == workspace) {
            Some(entry) => entry.1 = current_id,
            None => self.last_current.push((workspace, current_id)),
        }
        for tab in &mut self.tabs {
            if tab.last_active == 0 {
                tab.last_active = now;
            }
        }
        let tab = &mut self.tabs[current];
        tab.last_active = now;
        std::mem::take(&mut tab.suspended).then_some(current)
    }

    /// Background tabs that have not been current for `idle_secs` and are not
    /// suspended yet, in strip order. Blank tabs are left alone.
    pub fn idle_tabs(&self, now: u64, idle_secs: u64) -> Vec<usize> {
        self.tabs
            .iter()
            .enumerate()
            .filter(|&(i, tab)| {
                Some(i) != self.current()
                    && !tab.suspended
                    && !tab.url.is_empty()
                    && tab.last_active != 0
                    && now.saturating_sub(tab.last_active) >= idle_secs
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Suspend or wake the tab at `index`. The current tab cannot be
    /// suspended. True if the flag changed.
    pub fn set_suspended(&mut self, index: usize, suspended: bool) -> bool {
        if suspended && Some(index) == self.current() {
            return false;
        }
        match self.tabs.get_mut(index) {
            Some(tab) if tab.suspended != suspended => {
                tab.suspended = suspended;
                true
            }
            _ => false,
        }
    }

    /// Close every tab, without remembering them as closed tabs.
    pub fn clear(&mut self) {
        self.tabs.clear();
        self.current = 0;
    }

    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }

    pub fn workspace(&self, id: WorkspaceId) -> Option<&Workspace> {
        self.workspaces.iter().find(|w| w.id == id)
    }

    /// The workspace the strip shows: the current tab's.
    pub fn current_workspace(&self) -> WorkspaceId {
        match self.current() {
            Some(i) => self.tabs[i].workspace,
            None => self.empty_workspace,
        }
    }

    /// Indexes of the tabs in `workspace`, in strip order.
    pub fn rows_in(&self, workspace: WorkspaceId) -> Vec<usize> {
        (0..self.tabs.len())
            .filter(|&i| self.tabs[i].workspace == workspace)
            .collect()
    }

    /// Add a workspace after the others and return its id. A blank name
    /// becomes "Workspace N".
    pub fn add_workspace(&mut self, name: &str, color: &str) -> WorkspaceId {
        let id = self
            .workspaces
            .iter()
            .map(|w| w.id)
            .max()
            .map_or(0, |m| m + 1);
        let name = match name.trim() {
            "" => format!("Workspace {}", self.workspaces.len() + 1),
            name => name.to_owned(),
        };
        self.workspaces.push(Workspace {
            id,
            name,
            color: color.to_owned(),
            icon: String::new(),
        });
        id
    }

    /// Rename a workspace. False if there is no such workspace, the name is
    /// blank or unchanged.
    pub fn rename_workspace(&mut self, id: WorkspaceId, name: &str) -> bool {
        let name = name.trim();
        match self.workspaces.iter_mut().find(|w| w.id == id) {
            Some(w) if !name.is_empty() && w.name != name => {
                w.name = name.to_owned();
                true
            }
            _ => false,
        }
    }

    /// Change a workspace's color. False if there is no such workspace or the
    /// color is unchanged.
    pub fn set_workspace_color(&mut self, id: WorkspaceId, color: &str) -> bool {
        match self.workspaces.iter_mut().find(|w| w.id == id) {
            Some(w) if w.color != color => {
                w.color = color.to_owned();
                true
            }
            _ => false,
        }
    }

    /// Change a workspace's icon (trimmed, at most
    /// [`WORKSPACE_ICON_MAX_CHARS`] characters; empty shows the color dot).
    /// False if there is no such workspace or the icon is unchanged.
    pub fn set_workspace_icon(&mut self, id: WorkspaceId, icon: &str) -> bool {
        let icon: String = icon.trim().chars().take(WORKSPACE_ICON_MAX_CHARS).collect();
        match self.workspaces.iter_mut().find(|w| w.id == id) {
            Some(w) if w.icon != icon => {
                w.icon = icon;
                true
            }
            _ => false,
        }
    }

    /// Remove an empty workspace. False if it still has tabs, is the last
    /// workspace, or does not exist.
    pub fn remove_workspace(&mut self, id: WorkspaceId) -> bool {
        if self.workspaces.len() < 2 || self.tabs.iter().any(|t| t.workspace == id) {
            return false;
        }
        let Some(at) = self.workspaces.iter().position(|w| w.id == id) else {
            return false;
        };
        self.workspaces.remove(at);
        self.last_current.retain(|&(w, _)| w != id);
        if self.empty_workspace == id {
            self.empty_workspace = self.workspaces[0].id;
        }
        true
    }

    /// Move a workspace to position `to` in the switcher. False if nothing
    /// moves.
    pub fn move_workspace(&mut self, id: WorkspaceId, to: usize) -> bool {
        let Some(from) = self.workspaces.iter().position(|w| w.id == id) else {
            return false;
        };
        if to >= self.workspaces.len() || from == to {
            return false;
        }
        let w = self.workspaces.remove(from);
        self.workspaces.insert(to, w);
        true
    }

    /// The tab to show when switching to `id`: the one last current there,
    /// else its first tab. `None` for an empty or unknown workspace.
    pub fn workspace_tab(&self, id: WorkspaceId) -> Option<usize> {
        let remembered = self
            .last_current
            .iter()
            .find(|&&(w, _)| w == id)
            .and_then(|&(_, tab)| self.index_of(tab))
            .filter(|&i| self.tabs[i].workspace == id);
        remembered.or_else(|| self.rows_in(id).first().copied())
    }

    /// Move the tab at `index` into workspace `id`. True if it moved.
    pub fn move_to_workspace(&mut self, index: usize, id: WorkspaceId) -> bool {
        if self.workspace(id).is_none() {
            return false;
        }
        match self.tabs.get_mut(index) {
            Some(tab) if tab.workspace != id => {
                tab.workspace = id;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(urls: &[&str]) -> TabList {
        let mut list = TabList::new();
        for url in urls {
            list.open(list.len(), url, "", false);
        }
        list
    }

    fn urls(list: &TabList) -> Vec<&str> {
        list.tabs().iter().map(|t| t.url.as_str()).collect()
    }

    #[test]
    fn first_tab_becomes_current() {
        let mut l = TabList::new();
        assert_eq!(l.current(), None);
        l.open(0, "a", "", false);
        assert_eq!(l.current(), Some(0));
    }

    #[test]
    fn background_open_before_current_keeps_current_tab() {
        let mut l = list(&["a", "b"]);
        l.activate(1);
        l.open(0, "z", "", false);
        assert_eq!(urls(&l), ["z", "a", "b"]);
        assert_eq!(l.current(), Some(2));
    }

    #[test]
    fn open_activates_and_clamps_index() {
        let mut l = list(&["a"]);
        assert_eq!(l.open(99, "b", "", true), 1);
        assert_eq!(l.current(), Some(1));
    }

    #[test]
    fn ids_are_unique_and_stable() {
        let mut l = list(&["a", "b", "c"]);
        let id = l.get(2).unwrap().id;
        l.close(0);
        assert_eq!(l.index_of(id), Some(1));
        l.open(0, "d", "", false);
        let ids: std::collections::HashSet<_> = l.tabs().iter().map(|t| t.id).collect();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn closing_current_activates_right_neighbour_then_left_at_end() {
        let mut l = list(&["a", "b", "c"]);
        l.activate(1);
        l.close(1);
        assert_eq!(l.current().map(|i| &l.tabs()[i].url[..]), Some("c"));
        l.close(1);
        assert_eq!(l.current().map(|i| &l.tabs()[i].url[..]), Some("a"));
        l.close(0);
        assert_eq!(l.current(), None);
    }

    #[test]
    fn closing_before_current_keeps_current_tab() {
        let mut l = list(&["a", "b", "c"]);
        l.activate(2);
        l.close(0);
        assert_eq!(l.current(), Some(1));
        assert_eq!(l.get(1).unwrap().url, "c");
    }

    #[test]
    fn close_out_of_range_is_ignored() {
        let mut l = list(&["a"]);
        assert!(l.close(3).is_none());
        assert_eq!(l.len(), 1);
    }

    #[test]
    fn reopen_restores_position_and_activates() {
        let mut l = list(&["a", "b", "c"]);
        l.close(1);
        l.close(0);
        assert_eq!(l.closed_count(), 2);
        assert_eq!(l.next_reopen_index(), Some(0));
        assert_eq!(l.reopen_closed(), Some(0));
        assert_eq!(l.reopen_closed(), Some(1));
        assert_eq!(urls(&l), ["a", "b", "c"]);
        assert_eq!(l.current(), Some(1));
        assert_eq!(l.reopen_closed(), None);
    }

    #[test]
    fn reopen_clamps_to_end_and_skips_blank_tabs() {
        let mut l = list(&["a", "b", ""]);
        l.close(2);
        assert_eq!(l.closed_count(), 0);
        l.close(1);
        l.close(0);
        assert_eq!(l.next_reopen_index(), Some(0));
        assert_eq!(l.reopen_closed(), Some(0));
        assert_eq!(urls(&l), ["a"]);

        let mut l = list(&["a", "b", "c"]);
        l.close(2);
        l.clear();
        assert_eq!(l.next_reopen_index(), Some(0));
        assert_eq!(l.reopen_closed(), Some(0));
    }

    #[test]
    fn closed_stack_is_bounded() {
        let mut l = TabList::new();
        for i in 0..CLOSED_TABS_KEPT + 5 {
            l.open(0, &format!("u{i}"), "", false);
            l.close(0);
        }
        assert_eq!(l.closed_count(), CLOSED_TABS_KEPT);
        l.reopen_closed();
        assert_eq!(l.get(0).unwrap().url, format!("u{}", CLOSED_TABS_KEPT + 4));
    }

    #[test]
    fn cycle_wraps_both_ways() {
        let mut l = list(&["a", "b", "c"]);
        assert_eq!(l.cycle(-1), Some(2));
        assert_eq!(l.cycle(1), Some(0));
        assert_eq!(l.cycle(4), Some(1));
        assert_eq!(TabList::new().cycle(1), None);
    }

    #[test]
    fn move_keeps_current_tab_current() {
        let mut l = list(&["a", "b", "c", "d"]);
        l.activate(1);
        assert!(l.move_tab(0, 3));
        assert_eq!(urls(&l), ["b", "c", "d", "a"]);
        assert_eq!(l.current(), Some(0));
        assert!(l.move_tab(0, 2));
        assert_eq!(urls(&l), ["c", "d", "b", "a"]);
        assert_eq!(l.current(), Some(2));
        assert!(!l.move_tab(1, 1));
        assert!(!l.move_tab(0, 4));
    }

    #[test]
    fn setters_report_changes() {
        let mut l = list(&["a"]);
        assert!(l.set_url(0, "b"));
        assert!(!l.set_url(0, "b"));
        assert!(l.set_title(0, "B"));
        assert!(!l.set_title(0, "B"));
        assert!(!l.set_title(5, "x"));
        assert!(l.set_icon(0, "image://favicon/b"));
        assert!(!l.set_icon(0, "image://favicon/b"));
    }

    #[test]
    fn idle_background_tabs_are_found_and_waking_clears_suspension() {
        let mut l = list(&["a", "b", "c", ""]);
        l.note_current(100);
        assert!(l.idle_tabs(150, 60).is_empty());
        assert_eq!(l.idle_tabs(160, 60), [1, 2]);
        assert!(l.set_suspended(1, true));
        assert!(!l.set_suspended(1, true));
        assert!(!l.set_suspended(0, true), "the current tab stays awake");
        assert_eq!(l.idle_tabs(160, 60), [2]);
        l.activate(1);
        assert_eq!(l.note_current(170), Some(1));
        assert!(!l.get(1).unwrap().suspended);
        // "a" stopped being current at 170, so it is idle only from 230.
        assert_eq!(l.idle_tabs(229, 60), [2]);
        assert_eq!(l.idle_tabs(230, 60), [0, 2]);
        assert_eq!(l.note_current(231), None);
    }

    #[test]
    fn unstamped_tabs_are_never_idle() {
        let mut l = list(&["a"]);
        l.open(1, "b", "", false);
        assert!(l.idle_tabs(1_000_000, 60).is_empty());
        assert_eq!(TabList::new().note_current(5), None);
    }

    #[test]
    fn activate_rejects_out_of_range_and_current() {
        let mut l = list(&["a", "b"]);
        assert!(!l.activate(0));
        assert!(!l.activate(2));
        assert!(l.activate(1));
    }

    #[test]
    fn session_round_trip() {
        let mut l = list(&["a", "b", "c"]);
        l.set_title(1, "Bee");
        l.set_icon(1, "image://favicon/b.png");
        l.activate(1);
        let restored = TabList::from_session(&l.to_session());
        assert_eq!(restored.get(1).unwrap().icon, "image://favicon/b.png");
        assert_eq!(urls(&restored), ["a", "b", "c"]);
        assert_eq!(restored.get(1).unwrap().title, "Bee");
        assert_eq!(restored.current(), Some(1));
    }

    #[test]
    fn from_session_clamps_current() {
        let session = Session {
            tabs: vec![SavedTab {
                url: "a".into(),
                ..SavedTab::default()
            }],
            current: 7,
            ..Session::default()
        };
        assert_eq!(TabList::from_session(&session).current(), Some(0));
    }

    #[test]
    fn new_tabs_join_the_current_workspace() {
        let mut l = list(&["a"]);
        assert_eq!(l.workspaces().len(), 1);
        assert_eq!(l.workspaces()[0].name, DEFAULT_WORKSPACE_NAME);
        let work = l.add_workspace("Work", "#336699");
        let i = l.open_in(work, 1, "w1", "", true);
        assert_eq!(l.current_workspace(), work);
        l.open(i + 1, "w2", "", false);
        assert_eq!(l.rows_in(work), [1, 2]);
        assert_eq!(l.rows_in(0), [0]);
    }

    #[test]
    fn cycle_and_nth_stay_in_the_workspace() {
        let mut l = list(&["a", "b"]);
        let work = l.add_workspace("Work", "");
        l.open_in(work, 1, "w1", "", false);
        l.open_in(work, 3, "w2", "", false);
        // a, w1, b, w2: the default workspace has a and b.
        assert_eq!(l.cycle(1), Some(2));
        assert_eq!(l.cycle(1), Some(0));
        assert_eq!(l.cycle(-1), Some(2));
        assert_eq!(l.nth_in_workspace(0), Some(0));
        assert_eq!(l.nth_in_workspace(-1), Some(2));
        assert_eq!(l.nth_in_workspace(5), None);
        assert_eq!(l.neighbour(0, 1), Some(2));
        assert_eq!(l.neighbour(3, -1), Some(1));
        assert_eq!(l.neighbour(2, 1), None);
    }

    #[test]
    fn closing_current_prefers_a_tab_in_the_same_workspace() {
        let mut l = list(&["a", "b"]);
        let work = l.add_workspace("Work", "");
        l.open_in(work, 1, "w1", "", false);
        // a, w1, b; close a: b is next in the default workspace, past w1.
        l.close(0);
        assert_eq!(l.get(l.current().unwrap()).unwrap().url, "b");
        // Closing b, the last default tab, falls back to w1.
        l.close(l.current().unwrap());
        assert_eq!(l.get(l.current().unwrap()).unwrap().url, "w1");
    }

    #[test]
    fn switching_back_returns_to_the_last_current_tab() {
        let mut l = list(&["a", "b"]);
        l.activate(1);
        l.note_current(1);
        let work = l.add_workspace("Work", "");
        assert_eq!(l.workspace_tab(work), None);
        let w = l.open_in(work, 2, "w1", "", true);
        l.note_current(2);
        assert_eq!(l.workspace_tab(0), Some(1));
        l.activate(l.workspace_tab(0).unwrap());
        assert_eq!(l.workspace_tab(work), Some(w));
    }

    #[test]
    fn workspaces_rename_recolor_move_and_remove() {
        let mut l = list(&["a"]);
        let work = l.add_workspace("", "");
        assert_eq!(l.workspace(work).unwrap().name, "Workspace 2");
        assert!(l.rename_workspace(work, " Work "));
        assert!(!l.rename_workspace(work, "Work"));
        assert!(!l.rename_workspace(work, "  "));
        assert!(l.set_workspace_color(work, "#ff0000"));
        assert!(l.set_workspace_icon(work, " 🚀 "));
        assert_eq!(l.workspace(work).unwrap().icon, "🚀");
        assert!(!l.set_workspace_icon(work, "🚀"));
        assert!(l.set_workspace_icon(work, "abcdefghijk"));
        assert_eq!(l.workspace(work).unwrap().icon, "abcdefgh");
        assert!(l.move_workspace(work, 0));
        assert_eq!(l.workspaces()[0].id, work);
        l.open_in(work, 1, "w", "", false);
        assert!(!l.remove_workspace(work), "still has a tab");
        assert!(l.move_to_workspace(1, 0));
        assert!(l.remove_workspace(work));
        assert!(!l.remove_workspace(0), "the last workspace stays");
    }

    #[test]
    fn workspaces_survive_a_session_round_trip() {
        let mut l = list(&["a"]);
        let work = l.add_workspace("Work", "#336699");
        l.set_workspace_icon(work, "💼");
        l.open_in(work, 1, "w", "", true);
        let session = l.to_session();
        let json = session.to_json();
        let back = TabList::from_session(&Session::from_json(&json).unwrap());
        assert_eq!(back.workspaces(), l.workspaces());
        assert_eq!(back.current_workspace(), work);
        assert_eq!(back.rows_in(work), [1]);
        // Tabs naming a workspace that no longer exists land in the first.
        let mut odd = session.clone();
        odd.tabs[1].workspace = 42;
        assert_eq!(TabList::from_session(&odd).rows_in(0), [0, 1]);
        // Old files without workspaces load into the default one.
        let old = Session::from_json(r#"{"tabs":[{"url":"a"}]}"#).unwrap();
        assert_eq!(TabList::from_session(&old).workspaces().len(), 1);
    }
}
