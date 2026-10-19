//! The ordered list of open tabs.

use crate::session::{SavedGroup, SavedSplit, SavedTab, SavedWorkspace, Session};
use unicode_segmentation::UnicodeSegmentation;

/// Stable identity of a tab for as long as it is open. Indices shift as tabs
/// open, close and move; ids do not.
pub type TabId = u64;

/// Stable identity of a workspace, saved with the session.
pub type WorkspaceId = u32;

/// Stable identity of a tab group, saved with the session.
pub type GroupId = u32;

/// Stable identity of a split view while it exists.
pub type SplitId = u32;

/// Most tabs one split view shows at once.
/// How many tabs [`TabList::previous_tab`] remembers.
const RECENT_MAX: usize = 32;

pub const SPLIT_MAX_PANES: usize = 4;

/// Smallest share of the space a pane of a split view can be given.
pub const SPLIT_MIN_SIZE: f64 = 0.1;

/// Tabs shown side by side (or stacked) while any of them is current.
#[derive(Clone, Debug, PartialEq)]
pub struct Split {
    pub id: SplitId,
    /// In pane order, left to right (top to bottom when `vertical`).
    pub tabs: Vec<TabId>,
    /// Each pane's share of the space, summing to 1.
    pub sizes: Vec<f64>,
    pub vertical: bool,
}

/// `sizes` cut or padded to `n` panes, each at least [`SPLIT_MIN_SIZE`],
/// scaled to sum to 1. Anything unusable gives equal panes.
fn normalize_sizes(sizes: &[f64], n: usize) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    let usable = sizes.len() == n && sizes.iter().all(|s| s.is_finite() && *s > 0.0);
    if !usable {
        return vec![1.0 / n as f64; n];
    }
    let total: f64 = sizes.iter().sum();
    let mut out: Vec<f64> = sizes
        .iter()
        .map(|s| (s / total).max(SPLIT_MIN_SIZE))
        .collect();
    let total: f64 = out.iter().sum();
    out.iter_mut().for_each(|s| *s /= total);
    out
}

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

/// A run of neighbouring tabs in one workspace's strip, under a colored,
/// optionally named header that can fold the run away.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabGroup {
    pub id: GroupId,
    /// May be empty: the header then shows only its color.
    pub name: String,
    /// A `#rrggbb` color, or empty for the theme's accent.
    pub color: String,
    /// Folded: the strip shows only the header.
    pub collapsed: bool,
}

/// Longest workspace icon kept, in user-perceived characters (grapheme
/// clusters), so a multi-part emoji counts once and is never cut in half.
pub const WORKSPACE_ICON_MAX_CHARS: usize = 8;

/// Longest workspace icon kept, in bytes. One character can hold any number of
/// combining marks, so the character cap alone does not bound the size.
pub const WORKSPACE_ICON_MAX_BYTES: usize = 256;

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
    /// Kept at the front of its workspace's strip, as an icon.
    pub pinned: bool,
    /// Pinned in every workspace (always also `pinned`). Its `workspace` is
    /// ignored while it is one.
    pub essential: bool,
    /// The tab group it belongs to. Pinned tabs are never in one.
    pub group: Option<GroupId>,
}

impl Tab {
    /// Strip section: Essentials first, then pinned tabs, then the rest. The
    /// list is always sorted by it.
    fn rank(&self) -> u8 {
        if self.essential {
            0
        } else if self.pinned {
            1
        } else {
            2
        }
    }
}

/// How a tab is pinned, for [`TabList::pin`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pin {
    None,
    /// Pinned in its workspace.
    Workspace,
    /// An Essential, pinned in every workspace.
    Everywhere,
}

/// A recently closed tab and where it was.
#[derive(Clone, Debug)]
pub struct ClosedTab {
    pub tab: Tab,
    pub index: usize,
}

/// Open tabs in strip order, which one is current, the workspaces and groups
/// they are sorted into, and recently closed tabs.
///
/// Every mutating method reports what changed so the UI adapter can emit the
/// matching model signals. Methods that move a tab have a matching `*_target`
/// method that says where it will land, so the adapter can announce the move
/// first. `current` is always a valid index while the list is not empty, and
/// there is always at least one workspace.
///
/// Order invariants: Essentials come first, then pinned tabs, then the rest;
/// a group's tabs are neighbours among their workspace's tabs.
#[derive(Debug)]
pub struct TabList {
    tabs: Vec<Tab>,
    current: usize,
    /// The tab [`note_current`](Self::note_current) last saw as current.
    seen_current: Option<TabId>,
    next_id: TabId,
    closed: Vec<ClosedTab>,
    workspaces: Vec<Workspace>,
    /// The workspace shown while the current tab is an Essential, or no tab
    /// is open: the last one a workspace tab was current in.
    shown_workspace: WorkspaceId,
    /// Each workspace's last current tab, to return to when switching back.
    last_current: Vec<(WorkspaceId, TabId)>,
    /// Tabs in the order they were last current, most recent first.
    recent: Vec<TabId>,
    groups: Vec<TabGroup>,
    /// Bumped whenever a group is added, removed or changed.
    groups_revision: u64,
    splits: Vec<Split>,
    /// Bumped whenever a split view is added, removed or changed.
    splits_revision: u64,
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
            shown_workspace: 0,
            last_current: Vec::new(),
            recent: Vec::new(),
            groups: Vec::new(),
            groups_revision: 0,
            splits: Vec::new(),
            splits_revision: 0,
        }
    }
}

/// A workspace icon trimmed and cut to [`WORKSPACE_ICON_MAX_CHARS`] whole
/// characters and [`WORKSPACE_ICON_MAX_BYTES`] bytes.
fn clamp_icon(icon: &str) -> String {
    let mut kept = String::new();
    for g in icon.trim().graphemes(true).take(WORKSPACE_ICON_MAX_CHARS) {
        if kept.len() + g.len() > WORKSPACE_ICON_MAX_BYTES {
            break;
        }
        kept.push_str(g);
    }
    kept
}

/// One past the largest id in `ids`, or the smallest unused id once the
/// largest is `u32::MAX` (a hand-edited session can contain it).
fn next_free_id(ids: impl Iterator<Item = u32> + Clone) -> u32 {
    ids.clone()
        .max()
        .map_or(Some(0), |m| m.checked_add(1))
        .unwrap_or_else(|| {
            (0..u32::MAX)
                .find(|candidate| !ids.clone().any(|id| id == *candidate))
                .expect("an in-memory list cannot contain every u32 id")
        })
}

/// The better of a tab's title and address scores; the address is matched
/// without its scheme and counts one point less.
fn best_score(score: &impl Fn(&str) -> Option<i32>, title: &str, url: &str) -> Option<i32> {
    let address = url.split_once("://").map_or(url, |(_, rest)| rest);
    let address = address.strip_prefix("www.").unwrap_or(address);
    match (score(title), score(address).map(|s| s - 1)) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
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
                    icon: clamp_icon(&w.icon),
                })
                .collect();
            list.shown_workspace = list.workspaces[0].id;
        }
        list.groups = session
            .groups
            .iter()
            .map(|g| TabGroup {
                id: g.id,
                name: g.name.clone(),
                color: g.color.clone(),
                collapsed: g.collapsed,
            })
            .collect();
        let fallback = list.workspaces[0].id;
        for saved in &session.tabs {
            let id = list.next_id;
            list.next_id += 1;
            let essential = saved.essential;
            let pinned = saved.pinned || essential;
            list.tabs.push(Tab {
                id,
                url: saved.url.clone(),
                title: saved.title.clone(),
                icon: saved.icon.clone(),
                suspended: false,
                last_active: 0,
                workspace: if list.workspaces.iter().any(|w| w.id == saved.workspace) {
                    saved.workspace
                } else {
                    fallback
                },
                pinned,
                essential,
                group: saved
                    .group
                    .filter(|&g| !pinned && list.groups.iter().any(|x| x.id == g)),
            });
        }
        let saved_ids: Vec<TabId> = list.tabs.iter().map(|t| t.id).collect();
        for saved in &session.splits {
            let tabs: Vec<TabId> = saved
                .tabs
                .iter()
                .filter_map(|&i| saved_ids.get(i).copied())
                .take(SPLIT_MAX_PANES)
                .collect();
            let taken = |id: &TabId| list.splits.iter().any(|s| s.tabs.contains(id));
            if tabs.iter().any(taken) || tabs.iter().enumerate().any(|(i, t)| tabs[..i].contains(t))
            {
                continue;
            }
            let id = next_free_id(list.splits.iter().map(|s| s.id));
            list.splits.push(Split {
                id,
                sizes: normalize_sizes(&saved.sizes, tabs.len()),
                tabs,
                vertical: saved.vertical,
            });
        }
        list.prune_splits();
        let current_id = session
            .tabs
            .len()
            .checked_sub(1)
            .map(|last| list.tabs[session.current.min(last)].id);
        // Hand-edited files may break the order invariants: restore them.
        list.tabs.sort_by_key(Tab::rank);
        list.split_stray_group_members();
        list.prune_groups();
        list.current = current_id.and_then(|id| list.index_of(id)).unwrap_or(0);
        if let Some(shown) = session
            .workspace
            .filter(|&id| list.workspaces.iter().any(|w| w.id == id))
        {
            list.shown_workspace = shown;
        }
        list.track_current();
        list
    }

    /// A group member that is not next to the group's earlier members leaves
    /// the group, so every group is one run.
    fn split_stray_group_members(&mut self) {
        let mut previous: Vec<(GroupId, usize)> = Vec::new();
        for workspace in self.workspaces.iter().map(|w| w.id).collect::<Vec<_>>() {
            let rows = self.rows_in(workspace);
            for (at, &i) in rows.iter().enumerate() {
                let Some(g) = self.tabs[i].group else {
                    continue;
                };
                match previous.iter_mut().find(|(id, _)| *id == g) {
                    Some(entry) if entry.1 + 1 == at => entry.1 = at,
                    Some(_) => self.tabs[i].group = None,
                    None => previous.push((g, at)),
                }
            }
            previous.clear();
        }
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
                    pinned: t.pinned,
                    essential: t.essential,
                    group: t.group,
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
            groups: self
                .groups
                .iter()
                .map(|g| SavedGroup {
                    id: g.id,
                    name: g.name.clone(),
                    color: g.color.clone(),
                    collapsed: g.collapsed,
                })
                .collect(),
            splits: self
                .splits
                .iter()
                .map(|s| SavedSplit {
                    tabs: s.tabs.iter().filter_map(|&id| self.index_of(id)).collect(),
                    sizes: s.sizes.clone(),
                    vertical: s.vertical,
                })
                .collect(),
            workspace: self
                .tabs
                .get(self.current)
                .filter(|t| t.essential)
                .map(|_| self.shown_workspace),
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

    /// Recently closed tabs, most recent first.
    pub fn closed_tabs(&self) -> impl Iterator<Item = &ClosedTab> {
        self.closed.iter().rev()
    }

    /// Indexes `[start, end)` of the tabs of strip section `rank`.
    fn section(&self, rank: u8) -> (usize, usize) {
        let start = self.tabs.iter().take_while(|t| t.rank() < rank).count();
        let len = self.tabs[start..]
            .iter()
            .take_while(|t| t.rank() == rank)
            .count();
        (start, start + len)
    }

    /// Number of Essentials and pinned tabs, which come first.
    pub fn pinned_count(&self) -> usize {
        self.section(2).0
    }

    /// Where [`open`](Self::open) puts a tab asked for at `index`: after the
    /// pinned tabs, at most at the end.
    pub fn insert_index(&self, index: usize) -> usize {
        index.clamp(self.pinned_count(), self.tabs.len())
    }

    /// Open a tab in the current workspace at `index` (see
    /// [`insert_index`](Self::insert_index)) and return its index.
    pub fn open(&mut self, index: usize, url: &str, title: &str, activate: bool) -> usize {
        let workspace = self.current_workspace();
        self.open_in(workspace, index, url, title, activate)
    }

    /// Open a tab in `workspace` at `index` (see
    /// [`insert_index`](Self::insert_index)) and return its index.
    pub fn open_in(
        &mut self,
        workspace: WorkspaceId,
        index: usize,
        url: &str,
        title: &str,
        activate: bool,
    ) -> usize {
        let index = self.insert_index(index);
        let tab = Tab {
            id: self.next_id,
            url: url.to_owned(),
            title: title.to_owned(),
            icon: String::new(),
            suspended: false,
            last_active: 0,
            workspace,
            pinned: false,
            essential: false,
            group: None,
        };
        self.next_id += 1;
        self.insert(index, tab, activate)
    }

    /// Where [`open_next_to_current`](Self::open_next_to_current) puts its tab.
    pub fn next_to_current_index(&self) -> usize {
        self.insert_index(self.current().map_or(0, |i| i + 1))
    }

    /// Open a tab right after the current one (a link opened from its page),
    /// in the current tab's group if it has one. Returns its index.
    pub fn open_next_to_current(&mut self, url: &str, activate: bool) -> usize {
        let group = self.current().and_then(|i| self.tabs[i].group);
        let index = self.next_to_current_index();
        let index = self.open(index, url, "", activate);
        if group.is_some() {
            self.tabs[index].group = group;
            self.normalize_group(index);
        }
        index
    }

    fn insert(&mut self, index: usize, tab: Tab, activate: bool) -> usize {
        let was_empty = self.tabs.is_empty();
        self.tabs.insert(index, tab);
        if activate || was_empty {
            self.current = index;
        } else if index <= self.current {
            self.current += 1;
        }
        self.track_current();
        index
    }

    /// The workspace whose strip shows the tab at `index`: its own, or the
    /// shown one for an Essential.
    fn strip_of(&self, index: usize) -> WorkspaceId {
        let tab = &self.tabs[index];
        if tab.essential {
            self.current_workspace()
        } else {
            tab.workspace
        }
    }

    /// Close the tab at `index`. The current tab stays current; closing the
    /// current tab activates the next tab to its right in the same strip,
    /// else the nearest one to its left, else the nearest tab at all.
    pub fn close(&mut self, index: usize) -> Option<Tab> {
        if index >= self.tabs.len() {
            return None;
        }
        let strip = self.strip_of(index);
        let tab = self.tabs.remove(index);
        if index < self.current {
            self.current -= 1;
        } else if index == self.current && !self.tabs.is_empty() {
            let same = |t: &Tab| t.essential || t.workspace == strip;
            self.current = self.tabs[index..]
                .iter()
                .position(same)
                .map(|i| index + i)
                .or_else(|| self.tabs[..index].iter().rposition(same))
                .unwrap_or(index.min(self.tabs.len() - 1));
        }
        if self.tabs.is_empty() {
            self.current = 0;
            self.shown_workspace = strip;
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
        self.prune_groups();
        self.prune_splits();
        self.track_current();
        Some(tab)
    }

    /// Where [`reopen_closed`](Self::reopen_closed) would put the tab back.
    pub fn next_reopen_index(&self) -> Option<usize> {
        self.reopen_index_at(0)
    }

    /// Where [`reopen_closed_at`](Self::reopen_closed_at) would put the `n`th
    /// most recently closed tab back.
    pub fn reopen_index_at(&self, n: usize) -> Option<usize> {
        let at = self.closed.len().checked_sub(n + 1)?;
        let closed = &self.closed[at];
        let (lo, hi) = self.section(closed.tab.rank());
        Some(closed.index.clamp(lo, hi))
    }

    /// Reopen the most recently closed tab where it was and make it current.
    /// Returns its new index.
    pub fn reopen_closed(&mut self) -> Option<usize> {
        self.reopen_closed_at(0)
    }

    /// Reopen the `n`th most recently closed tab (0 is the latest) where it
    /// was and make it current. Returns its new index.
    pub fn reopen_closed_at(&mut self, n: usize) -> Option<usize> {
        let index = self.reopen_index_at(n)?;
        let at = self.closed.len() - n - 1;
        let ClosedTab { mut tab, .. } = self.closed.remove(at);
        tab.suspended = false;
        if self.workspace(tab.workspace).is_none() {
            tab.workspace = self.current_workspace();
        }
        let index = self.insert(index, tab, true);
        self.normalize_group(index);
        Some(index)
    }

    /// Make the tab at `index` current. False if out of range or already current.
    pub fn activate(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() || index == self.current {
            return false;
        }
        self.current = index;
        self.track_current();
        true
    }

    /// After the current tab changed: remember which workspace shows, and
    /// unfold the current tab's group.
    fn track_current(&mut self) {
        let Some(tab) = self.tabs.get(self.current) else {
            return;
        };
        if !tab.essential {
            self.shown_workspace = tab.workspace;
        }
        if let Some(g) = tab.group
            && let Some(group) = self.groups.iter_mut().find(|x| x.id == g && x.collapsed)
        {
            group.collapsed = false;
            self.groups_revision += 1;
        }
    }

    /// Indexes of the tabs the strip of `workspace` shows: the Essentials and
    /// the workspace's own tabs, in strip order.
    pub fn strip_rows(&self, workspace: WorkspaceId) -> Vec<usize> {
        (0..self.tabs.len())
            .filter(|&i| self.tabs[i].essential || self.tabs[i].workspace == workspace)
            .collect()
    }

    /// Move the current tab by `step` within its strip, wrapping around.
    /// Returns the new index.
    pub fn cycle(&mut self, step: isize) -> Option<usize> {
        let rows = self.strip_rows(self.current_workspace());
        let at = rows.iter().position(|&i| i == self.current)?;
        // Do the addition in two bounded pieces. `at + step` can overflow for
        // callers that use an extreme step even though the final, wrapped
        // index is always tiny.
        let offset = step.unsigned_abs() % rows.len();
        let next = if step < 0 {
            (at + rows.len() - offset) % rows.len()
        } else {
            (at + offset) % rows.len()
        };
        self.current = rows[next];
        self.track_current();
        Some(self.current)
    }

    /// Index of the tab `step` places from `index` among the tabs of its
    /// strip, without wrapping.
    pub fn neighbour(&self, index: usize, step: isize) -> Option<usize> {
        if index >= self.tabs.len() {
            return None;
        }
        let rows = self.strip_rows(self.strip_of(index));
        let at = rows.iter().position(|&i| i == index)? as isize + step;
        rows.get(usize::try_from(at).ok()?).copied()
    }

    /// Index of the `n`th tab of the current strip (0-based), or of its last
    /// tab when `n` is negative.
    pub fn nth_in_workspace(&self, n: isize) -> Option<usize> {
        let rows = self.strip_rows(self.current_workspace());
        if n < 0 {
            rows.last().copied()
        } else {
            rows.get(n as usize).copied()
        }
    }

    /// Where [`move_tab`](Self::move_tab) would put the tab at `from` asked to
    /// go to `to`: kept within its section (Essentials, pinned, the rest).
    /// `None` if either index is out of range or nothing would move.
    pub fn move_target(&self, from: usize, to: usize) -> Option<usize> {
        if from >= self.tabs.len() || to >= self.tabs.len() {
            return None;
        }
        let (lo, hi) = self.section(self.tabs[from].rank());
        let to = to.clamp(lo, hi - 1);
        (to != from).then_some(to)
    }

    /// Move the tab without touching groups or sections.
    fn raw_move(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        let current_id = self.tabs[self.current].id;
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        self.current = self.index_of(current_id).unwrap_or(0);
    }

    /// Move the tab at `from` so it ends up at `to` (see
    /// [`move_target`](Self::move_target)), joining or leaving groups by where
    /// it lands. The current tab stays the same tab. False if nothing moves.
    pub fn move_tab(&mut self, from: usize, to: usize) -> bool {
        let Some(to) = self.move_target(from, to) else {
            return false;
        };
        self.raw_move(from, to);
        self.drop_tab(to);
        true
    }

    /// One step of dragging a tab along the strip: like
    /// [`move_tab`](Self::move_tab), but groups are left alone until
    /// [`drop_tab`](Self::drop_tab), so a tab dragged across a group passes
    /// it instead of joining it.
    pub fn drag_tab(&mut self, from: usize, to: usize) -> bool {
        let Some(to) = self.move_target(from, to) else {
            return false;
        };
        self.raw_move(from, to);
        true
    }

    /// A dragged tab was let go at `index`: it joins the group it landed in,
    /// or leaves its own if it landed outside. True if its group changed.
    pub fn drop_tab(&mut self, index: usize) -> bool {
        let Some(before) = self.tabs.get(index).map(|t| t.group) else {
            return false;
        };
        self.normalize_group(index);
        self.prune_groups();
        self.tabs[index].group != before
    }

    /// Settle the group of the tab at `index` from its neighbours in its
    /// workspace: it keeps its group next to another of its members (or when
    /// it is the group's only tab), joins a group it sits inside, and
    /// otherwise belongs to none.
    fn normalize_group(&mut self, index: usize) {
        let tab = &self.tabs[index];
        if tab.pinned {
            self.tabs[index].group = None;
            return;
        }
        let rows = self.rows_in(tab.workspace);
        let at = rows.iter().position(|&r| r == index).unwrap_or(0);
        let group_of = |r: Option<&usize>| r.and_then(|&r| self.tabs[r].group);
        let left = group_of(at.checked_sub(1).and_then(|a| rows.get(a)));
        let right = group_of(rows.get(at + 1));
        let inside = left.is_some() && left == right;
        let own = tab.group.filter(|&g| self.group(g).is_some());
        let alone = own.is_some_and(|g| {
            self.tabs
                .iter()
                .enumerate()
                .all(|(i, t)| i == index || t.group != Some(g))
        });
        let group = match own {
            Some(g) if left == Some(g) || right == Some(g) => Some(g),
            Some(g) if alone && !inside => Some(g),
            _ if inside => left,
            _ => None,
        };
        self.tabs[index].group = group;
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
    /// the tab that was current before (if it changed) and the tabs on screen
    /// are stamped as active now, tabs never stamped get `now`, and the tabs
    /// on screen (the current one and the rest of its split view) are no
    /// longer suspended. Returns the rows whose suspended flag that cleared.
    pub fn note_current(&mut self, now: u64) -> Vec<usize> {
        let Some(current) = self.current() else {
            return Vec::new();
        };
        let current_id = self.tabs[current].id;
        if let Some(previous) = self.seen_current.filter(|&id| id != current_id)
            && let Some(i) = self.index_of(previous)
        {
            self.tabs[i].last_active = now;
        }
        if self.seen_current != Some(current_id) {
            self.recent.retain(|&id| id != current_id);
            self.recent.insert(0, current_id);
            self.recent.truncate(RECENT_MAX);
        }
        self.seen_current = Some(current_id);
        if !self.tabs[current].essential {
            let workspace = self.tabs[current].workspace;
            match self.last_current.iter_mut().find(|(w, _)| *w == workspace) {
                Some(entry) => entry.1 = current_id,
                None => self.last_current.push((workspace, current_id)),
            }
        }
        for tab in &mut self.tabs {
            if tab.last_active == 0 {
                tab.last_active = now;
            }
        }
        let shown: Vec<usize> = (0..self.tabs.len())
            .filter(|&i| self.on_screen(i))
            .collect();
        shown
            .into_iter()
            .filter(|&i| {
                let tab = &mut self.tabs[i];
                tab.last_active = now;
                std::mem::take(&mut tab.suspended)
            })
            .collect()
    }

    /// The tab current before the current one, among the tabs of the strip
    /// on screen; `None` if there is none. Follows
    /// [`note_current`](Self::note_current).
    pub fn previous_tab(&self) -> Option<usize> {
        let current = self.current()?;
        let workspace = self.current_workspace();
        self.recent
            .iter()
            .filter_map(|&id| self.index_of(id))
            .find(|&i| {
                i != current && (self.tabs[i].essential || self.tabs[i].workspace == workspace)
            })
    }

    /// Whether the tab at `index` is on screen: the current tab, or another
    /// pane of its split view.
    pub fn on_screen(&self, index: usize) -> bool {
        Some(index) == self.current()
            || self
                .current_split()
                .zip(self.tabs.get(index))
                .is_some_and(|(split, tab)| split.tabs.contains(&tab.id))
    }

    /// Background tabs that have not been on screen for `idle_secs` and are
    /// not suspended yet, in strip order. Blank tabs are left alone.
    pub fn idle_tabs(&self, now: u64, idle_secs: u64) -> Vec<usize> {
        self.tabs
            .iter()
            .enumerate()
            .filter(|&(i, tab)| {
                !self.on_screen(i)
                    && !tab.suspended
                    && !tab.url.is_empty()
                    && tab.last_active != 0
                    && now.saturating_sub(tab.last_active) >= idle_secs
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Suspend or wake the tab at `index`. Tabs on screen (the current one
    /// and its split view) cannot be suspended. True if the flag changed.
    pub fn set_suspended(&mut self, index: usize, suspended: bool) -> bool {
        if suspended && self.on_screen(index) {
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
        self.prune_groups();
        self.prune_splits();
    }

    /// Where [`pin`](Self::pin) moves the tab at `index`: the end of the
    /// Essentials, the end of the pinned tabs, or the start of the rest.
    pub fn pin_target(&self, index: usize, pin: Pin) -> Option<usize> {
        let tab = self.tabs.get(index)?;
        let rank = match pin {
            Pin::Everywhere => 0,
            Pin::Workspace => 1,
            Pin::None => 2,
        };
        let (lo, hi) = self.section(rank);
        Some(match rank.cmp(&tab.rank()) {
            std::cmp::Ordering::Equal => index,
            // Moving forward, into an earlier section: its end.
            std::cmp::Ordering::Less => hi,
            // Moving back: the start of the later section, which shifts down
            // by one once the tab leaves its own.
            std::cmp::Ordering::Greater => lo - 1,
        })
    }

    /// Pin the tab at `index` to its workspace, make it an Essential (pinned
    /// in every workspace), or unpin it, moving it to that section. An
    /// unpinned Essential joins the current workspace. Returns its new index,
    /// or `None` if there is no such tab.
    pub fn pin(&mut self, index: usize, pin: Pin) -> Option<usize> {
        let to = self.pin_target(index, pin)?;
        let workspace = self.current_workspace();
        let tab = &mut self.tabs[index];
        if tab.essential && pin != Pin::Everywhere {
            tab.workspace = workspace;
        }
        tab.pinned = pin != Pin::None;
        tab.essential = pin == Pin::Everywhere;
        tab.group = None;
        self.raw_move(index, to);
        self.prune_groups();
        self.track_current();
        Some(to)
    }

    /// Open tabs in every workspace that `score` accepts by title or
    /// address, best first: higher scores, then the most recently used. The
    /// address counts a little less than the title, and without its scheme.
    pub fn search(&self, score: impl Fn(&str) -> Option<i32>) -> Vec<usize> {
        let mut hits: Vec<(i32, u64, usize)> = self
            .tabs
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                let s = best_score(&score, &t.title, &t.url)?;
                let recency = if Some(i) == self.current() {
                    u64::MAX
                } else {
                    t.last_active
                };
                Some((s, recency, i))
            })
            .collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
        hits.into_iter().map(|(_, _, i)| i).collect()
    }

    /// Recently closed tabs `score` accepts, as positions for
    /// [`reopen_closed_at`](Self::reopen_closed_at), best first and then the
    /// most recently closed.
    pub fn search_closed(&self, score: impl Fn(&str) -> Option<i32>) -> Vec<usize> {
        let mut hits: Vec<(i32, usize)> = self
            .closed_tabs()
            .enumerate()
            .filter_map(|(n, c)| Some((best_score(&score, &c.tab.title, &c.tab.url)?, n)))
            .collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        hits.into_iter().map(|(_, n)| n).collect()
    }

    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }

    pub fn workspace(&self, id: WorkspaceId) -> Option<&Workspace> {
        self.workspaces.iter().find(|w| w.id == id)
    }

    /// The workspace the strip shows: the current tab's, or for an Essential
    /// the one shown before it.
    pub fn current_workspace(&self) -> WorkspaceId {
        match self.tabs.get(self.current) {
            Some(tab) if !tab.essential => tab.workspace,
            _ => self.shown_workspace,
        }
    }

    /// Indexes of the tabs in `workspace` (Essentials excluded), in strip
    /// order.
    pub fn rows_in(&self, workspace: WorkspaceId) -> Vec<usize> {
        (0..self.tabs.len())
            .filter(|&i| !self.tabs[i].essential && self.tabs[i].workspace == workspace)
            .collect()
    }

    /// Add a workspace after the others and return its id. A blank name
    /// becomes "Workspace N".
    pub fn add_workspace(&mut self, name: &str, color: &str) -> WorkspaceId {
        let id = next_free_id(self.workspaces.iter().map(|w| w.id));
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
    /// [`WORKSPACE_ICON_MAX_CHARS`] whole characters and
    /// [`WORKSPACE_ICON_MAX_BYTES`] bytes; empty shows the color dot).
    /// False if there is no such workspace or the icon is unchanged.
    pub fn set_workspace_icon(&mut self, id: WorkspaceId, icon: &str) -> bool {
        let icon = clamp_icon(icon);
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
        if self.workspaces.len() < 2 || !self.rows_in(id).is_empty() {
            return false;
        }
        let Some(at) = self.workspaces.iter().position(|w| w.id == id) else {
            return false;
        };
        self.workspaces.remove(at);
        self.last_current.retain(|&(w, _)| w != id);
        if self.shown_workspace == id {
            self.shown_workspace = self.workspaces[0].id;
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
            .filter(|&i| !self.tabs[i].essential && self.tabs[i].workspace == id);
        remembered.or_else(|| self.rows_in(id).first().copied())
    }

    /// Where [`move_to_workspace`](Self::move_to_workspace) puts the tab at
    /// `index`: the end of its section, so it lands at the end of the other
    /// workspace's strip.
    pub fn workspace_move_target(&self, index: usize) -> Option<usize> {
        let tab = self.tabs.get(index)?;
        Some(self.section(tab.rank()).1 - 1)
    }

    /// Move the tab at `index` into workspace `id`, to the end of its strip
    /// and out of its group. Essentials belong to every workspace and stay.
    /// Returns its new index if it moved.
    pub fn move_to_workspace(&mut self, index: usize, id: WorkspaceId) -> Option<usize> {
        self.workspace(id)?;
        let to = self.workspace_move_target(index)?;
        let tab = &mut self.tabs[index];
        if tab.essential || tab.workspace == id {
            return None;
        }
        tab.workspace = id;
        tab.group = None;
        // A split view shows one workspace's tabs.
        self.unsplit(index);
        self.raw_move(index, to);
        self.prune_groups();
        self.track_current();
        Some(to)
    }

    pub fn groups(&self) -> &[TabGroup] {
        &self.groups
    }

    pub fn group(&self, id: GroupId) -> Option<&TabGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// Changes whenever a group is added, removed or changed.
    pub fn groups_revision(&self) -> u64 {
        self.groups_revision
    }

    /// Indexes of the tabs in group `id`, in strip order.
    pub fn group_rows(&self, id: GroupId) -> Vec<usize> {
        (0..self.tabs.len())
            .filter(|&i| self.tabs[i].group == Some(id))
            .collect()
    }

    /// Drop groups that have no tabs left.
    fn prune_groups(&mut self) {
        let before = self.groups.len();
        let tabs = &self.tabs;
        self.groups
            .retain(|g| tabs.iter().any(|t| t.group == Some(g.id)));
        if self.groups.len() != before {
            self.groups_revision += 1;
        }
    }

    /// Where [`set_group`](Self::set_group) moves the tab at `index`: after
    /// the last tab of the group it joins, or out of the middle of the group
    /// it leaves, so groups stay in one piece.
    pub fn group_target(&self, index: usize, group: Option<GroupId>) -> Option<usize> {
        let tab = self.tabs.get(index)?;
        if tab.pinned || tab.group == group {
            return Some(index);
        }
        let after = |members: Vec<usize>| match members.last() {
            Some(&last) if last > index => last,
            Some(&last) => last + 1,
            None => index,
        };
        if let Some(g) = group {
            let members = self.group_rows(g);
            if !members.is_empty() {
                return Some(after(members));
            }
        }
        // Leaving its group: from either end it can stay where it is.
        let Some(own) = tab.group else {
            return Some(index);
        };
        let rows = self.rows_in(tab.workspace);
        let at = rows.iter().position(|&r| r == index)?;
        let left = at.checked_sub(1).and_then(|a| self.tabs[rows[a]].group);
        let right = rows.get(at + 1).and_then(|&r| self.tabs[r].group);
        if left == Some(own) && right == Some(own) {
            Some(after(self.group_rows(own)))
        } else {
            Some(index)
        }
    }

    /// Put the tab at `index` into `group` (into its workspace, too), or take
    /// it out of its group with `None`. Pinned tabs are never grouped.
    /// Returns its new index.
    pub fn set_group(&mut self, index: usize, group: Option<GroupId>) -> Option<usize> {
        let group = group.filter(|&g| self.group(g).is_some());
        let to = self.group_target(index, group)?;
        if self.tabs[index].pinned {
            return Some(index);
        }
        if let Some(g) = group
            && let Some(&member) = self.group_rows(g).first()
        {
            self.tabs[index].workspace = self.tabs[member].workspace;
        }
        // Leaving first means `to` is computed against the group as it was.
        self.raw_move(index, to);
        self.tabs[to].group = group;
        self.prune_groups();
        self.track_current();
        Some(to)
    }

    /// Start a new group with the tab at `index` (see
    /// [`group_target`](Self::group_target) with `None` for where it moves).
    /// Returns the group's id and the tab's new index; `None` for a pinned or
    /// missing tab.
    pub fn create_group(
        &mut self,
        index: usize,
        name: &str,
        color: &str,
    ) -> Option<(GroupId, usize)> {
        if self.tabs.get(index)?.pinned {
            return None;
        }
        let to = self.set_group(index, None)?;
        let id = next_free_id(self.groups.iter().map(|g| g.id));
        self.groups.push(TabGroup {
            id,
            name: name.trim().to_owned(),
            color: color.to_owned(),
            collapsed: false,
        });
        self.groups_revision += 1;
        self.tabs[to].group = Some(id);
        Some((id, to))
    }

    /// Rename a group (blank is allowed: only the color shows). False if
    /// there is no such group or the name is unchanged.
    pub fn rename_group(&mut self, id: GroupId, name: &str) -> bool {
        let name = name.trim();
        match self.groups.iter_mut().find(|g| g.id == id) {
            Some(g) if g.name != name => {
                g.name = name.to_owned();
                self.groups_revision += 1;
                true
            }
            _ => false,
        }
    }

    /// Change a group's color. False if there is no such group or the color is
    /// unchanged.
    pub fn set_group_color(&mut self, id: GroupId, color: &str) -> bool {
        match self.groups.iter_mut().find(|g| g.id == id) {
            Some(g) if g.color != color => {
                g.color = color.to_owned();
                self.groups_revision += 1;
                true
            }
            _ => false,
        }
    }

    /// Fold or unfold a group. Folding the current tab's group first moves to
    /// the nearest tab outside it; with none, the group stays open. True if
    /// the group changed.
    pub fn set_group_collapsed(&mut self, id: GroupId, collapsed: bool) -> bool {
        let Some(group) = self.group(id) else {
            return false;
        };
        if group.collapsed == collapsed {
            return false;
        }
        if collapsed
            && self
                .current()
                .is_some_and(|c| self.tabs[c].group == Some(id))
        {
            let rows = self.strip_rows(self.current_workspace());
            let at = rows.iter().position(|&r| r == self.current).unwrap_or(0);
            let outside = |&&r: &&usize| self.tabs[r].group != Some(id);
            let Some(&next) = rows[at..]
                .iter()
                .find(outside)
                .or_else(|| rows[..at].iter().rev().find(outside))
            else {
                return false;
            };
            self.current = next;
            self.track_current();
        }
        if let Some(group) = self.groups.iter_mut().find(|g| g.id == id) {
            group.collapsed = collapsed;
        }
        self.groups_revision += 1;
        true
    }

    /// Dissolve a group, keeping its tabs where they are. False if there is no
    /// such group.
    pub fn ungroup(&mut self, id: GroupId) -> bool {
        if self.group(id).is_none() {
            return false;
        }
        for tab in &mut self.tabs {
            if tab.group == Some(id) {
                tab.group = None;
            }
        }
        self.prune_groups();
        true
    }

    pub fn splits(&self) -> &[Split] {
        &self.splits
    }

    /// Changes whenever a split view is added, removed or changed.
    pub fn splits_revision(&self) -> u64 {
        self.splits_revision
    }

    /// The split view showing the tab at `index`, if any.
    pub fn split_of(&self, index: usize) -> Option<&Split> {
        let id = self.tabs.get(index)?.id;
        self.splits.iter().find(|s| s.tabs.contains(&id))
    }

    /// The split view on screen: the current tab's.
    pub fn current_split(&self) -> Option<&Split> {
        self.split_of(self.current()?)
    }

    /// Drop closed tabs from split views and views left with one tab.
    fn prune_splits(&mut self) {
        let before = self.splits.clone();
        let open: Vec<TabId> = self.tabs.iter().map(|t| t.id).collect();
        for split in &mut self.splits {
            let keep: Vec<bool> = split.tabs.iter().map(|id| open.contains(id)).collect();
            if keep.iter().all(|&k| k) {
                continue;
            }
            let sizes: Vec<f64> = split
                .sizes
                .iter()
                .zip(&keep)
                .filter(|&(_, &k)| k)
                .map(|(s, _)| *s)
                .collect();
            split.tabs.retain(|id| open.contains(id));
            split.sizes = normalize_sizes(&sizes, split.tabs.len());
        }
        self.splits.retain(|s| s.tabs.len() > 1);
        if self.splits != before {
            self.splits_revision += 1;
        }
    }

    /// Show the tab at `other` next to the tab at `index`: in `index`'s split
    /// view right after it, or in a new one of the two. `other` leaves any
    /// split it was in and joins `index`'s workspace. Panes get equal space.
    /// Returns the split's id; `None` if either tab is missing, they are the
    /// same, or the split already has [`SPLIT_MAX_PANES`].
    pub fn split_tabs(&mut self, index: usize, other: usize) -> Option<SplitId> {
        let (a, b) = (self.tabs.get(index)?.id, self.tabs.get(other)?.id);
        if a == b || self.split_of(index).is_some_and(|s| s.tabs.contains(&b)) {
            return self.split_of(index).map(|s| s.id);
        }
        if self
            .split_of(index)
            .is_some_and(|s| s.tabs.len() >= SPLIT_MAX_PANES)
        {
            return None;
        }
        self.unsplit(other);
        let (workspace, anchor_essential) =
            (self.tabs[index].workspace, self.tabs[index].essential);
        let joining = &mut self.tabs[other];
        if !joining.essential && !anchor_essential && joining.workspace != workspace {
            joining.workspace = workspace;
            joining.group = None;
        }
        self.normalize_group(other);
        let id = match self.splits.iter_mut().find(|s| s.tabs.contains(&a)) {
            Some(split) => {
                let at = split.tabs.iter().position(|&t| t == a).unwrap_or(0);
                split.tabs.insert(at + 1, b);
                split.sizes = normalize_sizes(&[], split.tabs.len());
                split.id
            }
            None => {
                let id = next_free_id(self.splits.iter().map(|s| s.id));
                self.splits.push(Split {
                    id,
                    tabs: vec![a, b],
                    sizes: normalize_sizes(&[], 2),
                    vertical: false,
                });
                id
            }
        };
        self.prune_groups();
        self.splits_revision += 1;
        Some(id)
    }

    /// Take the tab at `index` out of its split view; a view left with one
    /// tab ends. True if it was in one.
    pub fn unsplit(&mut self, index: usize) -> bool {
        let Some(id) = self.tabs.get(index).map(|t| t.id) else {
            return false;
        };
        let Some(split) = self.splits.iter_mut().find(|s| s.tabs.contains(&id)) else {
            return false;
        };
        let at = split.tabs.iter().position(|&t| t == id).unwrap_or(0);
        split.tabs.remove(at);
        let mut sizes = split.sizes.clone();
        if at < sizes.len() {
            sizes.remove(at);
        }
        split.sizes = normalize_sizes(&sizes, split.tabs.len());
        self.splits.retain(|s| s.tabs.len() > 1);
        self.splits_revision += 1;
        true
    }

    /// End split view `id`, keeping its tabs. False if there is none.
    pub fn end_split(&mut self, id: SplitId) -> bool {
        let before = self.splits.len();
        self.splits.retain(|s| s.id != id);
        if self.splits.len() == before {
            return false;
        }
        self.splits_revision += 1;
        true
    }

    /// Set the panes' shares of the space (see [`SPLIT_MIN_SIZE`]). False if
    /// there is no such split, the count is wrong or nothing changes.
    pub fn set_split_sizes(&mut self, id: SplitId, sizes: &[f64]) -> bool {
        let Some(split) = self.splits.iter_mut().find(|s| s.id == id) else {
            return false;
        };
        if sizes.len() != split.tabs.len() {
            return false;
        }
        let sizes = normalize_sizes(sizes, split.tabs.len());
        if sizes == split.sizes {
            return false;
        }
        split.sizes = sizes;
        self.splits_revision += 1;
        true
    }

    /// Stack the panes top to bottom, or put them side by side. False if
    /// there is no such split or nothing changes.
    pub fn set_split_vertical(&mut self, id: SplitId, vertical: bool) -> bool {
        match self.splits.iter_mut().find(|s| s.id == id) {
            Some(split) if split.vertical != vertical => {
                split.vertical = vertical;
                self.splits_revision += 1;
                true
            }
            _ => false,
        }
    }

    /// Swap the pane showing the tab at `index` with its neighbour `step`
    /// places along (−1 or 1). False at either end.
    pub fn move_pane(&mut self, index: usize, step: isize) -> bool {
        let Some(id) = self.tabs.get(index).map(|t| t.id) else {
            return false;
        };
        let Some(split) = self.splits.iter_mut().find(|s| s.tabs.contains(&id)) else {
            return false;
        };
        let at = split.tabs.iter().position(|&t| t == id).unwrap_or(0);
        let Some(to) = at
            .checked_add_signed(step)
            .filter(|&to| to < split.tabs.len())
        else {
            return false;
        };
        split.tabs.swap(at, to);
        split.sizes.swap(at, to);
        self.splits_revision += 1;
        true
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
        assert_eq!(l.cycle(isize::MAX), Some(2));
        assert_eq!(l.cycle(isize::MIN), Some(0));
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
        assert_eq!(l.note_current(170), [1]);
        assert!(!l.get(1).unwrap().suspended);
        // "a" stopped being current at 170, so it is idle only from 230.
        assert_eq!(l.idle_tabs(229, 60), [2]);
        assert_eq!(l.idle_tabs(230, 60), [0, 2]);
        assert!(l.note_current(231).is_empty());
    }

    #[test]
    fn unstamped_tabs_are_never_idle() {
        let mut l = list(&["a"]);
        l.open(1, "b", "", false);
        assert!(l.idle_tabs(1_000_000, 60).is_empty());
        assert!(TabList::new().note_current(5).is_empty());
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
    fn essential_current_keeps_its_workspace_across_restarts() {
        let mut l = list(&["a", "b"]);
        let work = l.add_workspace("Work", "");
        l.open_in(work, 2, "w", "", true);
        let e = l.pin(0, Pin::Everywhere).unwrap();
        l.activate(e);
        assert_eq!(l.current_workspace(), work);
        let restored = TabList::from_session(&l.to_session());
        assert_eq!(restored.current_workspace(), work);
        assert!(restored.get(restored.current().unwrap()).unwrap().essential);
    }

    #[test]
    fn previous_tab_is_the_last_one_current_in_this_strip() {
        let mut l = list(&["a", "b", "c"]);
        let work = l.add_workspace("Work", "");
        l.open_in(work, 3, "w", "", false);
        for i in [0, 1, 2] {
            l.activate(i);
            // All in the same second: order still counts.
            l.note_current(5);
        }
        assert_eq!(l.previous_tab(), Some(1));
        l.activate(0);
        l.note_current(5);
        assert_eq!(l.previous_tab(), Some(2));
        l.activate(3);
        l.note_current(5);
        assert_eq!(l.previous_tab(), None);
    }

    #[test]
    fn moving_a_pane_to_another_workspace_takes_it_out_of_the_split() {
        let mut l = list(&["a", "b", "c"]);
        let work = l.add_workspace("Work", "");
        l.split_tabs(0, 1).unwrap();
        l.split_tabs(0, 2).unwrap();
        let to = l.move_to_workspace(2, work).unwrap();
        assert!(l.split_of(to).is_none());
        assert_eq!(l.split_of(0).unwrap().tabs.len(), 2);
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
    fn adding_a_workspace_after_the_largest_id_reuses_a_gap() {
        let session = Session {
            workspaces: vec![
                SavedWorkspace {
                    id: 0,
                    name: "Default".into(),
                    ..SavedWorkspace::default()
                },
                SavedWorkspace {
                    id: u32::MAX,
                    name: "Imported".into(),
                    ..SavedWorkspace::default()
                },
            ],
            ..Session::default()
        };
        let mut list = TabList::from_session(&session);
        let id = list.add_workspace("Work", "");
        assert_eq!(id, 1);
        assert_eq!(list.workspaces().iter().filter(|w| w.id == id).count(), 1);
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
        // A family emoji is seven scalars but one character: kept whole.
        let family = "👨\u{200d}👩\u{200d}👧\u{200d}👦";
        assert!(l.set_workspace_icon(work, &format!("{family}{family}")));
        assert_eq!(l.workspace(work).unwrap().icon, format!("{family}{family}"));
        // One character stuffed with combining marks is dropped, not kept whole.
        let zalgo = format!("a{}b", "\u{301}".repeat(1000));
        assert!(l.set_workspace_icon(work, &zalgo));
        assert_eq!(l.workspace(work).unwrap().icon, "");
        assert!(l.move_workspace(work, 0));
        assert_eq!(l.workspaces()[0].id, work);
        l.open_in(work, 1, "w", "", false);
        assert!(!l.remove_workspace(work), "still has a tab");
        assert!(l.move_to_workspace(1, 0).is_some());
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
        // Hand-edited oversized icons are cut down on load.
        let mut big = session.clone();
        big.workspaces[1].icon = "x".repeat(100);
        let restored = TabList::from_session(&big);
        assert_eq!(restored.workspace(work).unwrap().icon, "xxxxxxxx");
        // Old files without workspaces load into the default one.
        let old = Session::from_json(r#"{"tabs":[{"url":"a"}]}"#).unwrap();
        assert_eq!(TabList::from_session(&old).workspaces().len(), 1);
    }

    fn groups_of(list: &TabList) -> Vec<Option<GroupId>> {
        list.tabs().iter().map(|t| t.group).collect()
    }

    #[test]
    fn pinned_tabs_and_essentials_stay_in_front() {
        let mut l = list(&["a", "b", "c", "d"]);
        assert_eq!(l.pin_target(2, Pin::Workspace), Some(0));
        assert_eq!(l.pin(2, Pin::Workspace), Some(0));
        assert_eq!(urls(&l), ["c", "a", "b", "d"]);
        assert_eq!(l.pin(3, Pin::Everywhere), Some(0));
        assert_eq!(urls(&l), ["d", "c", "a", "b"]);
        assert_eq!(l.pinned_count(), 2);
        // New tabs never land among them, and pinned tabs can't be dragged out.
        assert_eq!(l.open(0, "e", "", false), 2);
        assert_eq!(l.move_target(1, 4), None);
        assert_eq!(l.move_target(4, 0), Some(2));
        // Unpinning puts the tab at the start of the unpinned tabs.
        assert_eq!(l.pin_target(0, Pin::None), Some(1));
        assert_eq!(l.pin(0, Pin::None), Some(1));
        assert_eq!(urls(&l), ["c", "d", "e", "a", "b"]);
        assert_eq!(l.pin(0, Pin::Everywhere), Some(0));
        assert_eq!(l.pin(0, Pin::Workspace), Some(0));
        assert!(l.get(0).unwrap().pinned && !l.get(0).unwrap().essential);
    }

    #[test]
    fn essentials_show_in_every_workspace_without_switching_it() {
        let mut l = list(&["mail", "a"]);
        l.pin(0, Pin::Everywhere);
        let work = l.add_workspace("Work", "");
        let w = l.open_in(work, 9, "w", "", true);
        assert_eq!(l.current_workspace(), work);
        assert_eq!(l.strip_rows(work), [0, w]);
        assert_eq!(l.strip_rows(0), [0, 1]);
        // Showing the Essential keeps the Work strip.
        l.activate(0);
        assert_eq!(l.current_workspace(), work);
        assert_eq!(l.cycle(1), Some(w));
        assert_eq!(l.rows_in(work), [w]);
        // Deleting a workspace never takes the Essentials with it.
        assert!(!l.remove_workspace(work));
        assert!(l.move_to_workspace(0, work).is_none());
        // Closing the current Essential stays in the shown strip.
        l.activate(0);
        l.close(0);
        assert_eq!(l.get(l.current().unwrap()).unwrap().url, "w");
    }

    #[test]
    fn pins_survive_a_session_round_trip_and_bad_order_is_fixed() {
        let mut l = list(&["a", "b", "c"]);
        l.pin(2, Pin::Everywhere);
        l.pin(2, Pin::Workspace);
        l.activate(2);
        let back = TabList::from_session(&l.to_session());
        assert_eq!(urls(&back), ["c", "b", "a"]);
        assert!(back.get(0).unwrap().essential);
        assert!(back.get(1).unwrap().pinned);
        assert_eq!(back.current(), Some(2));
        let mut odd = l.to_session();
        odd.tabs.reverse();
        odd.current = 0;
        let fixed = TabList::from_session(&odd);
        assert_eq!(urls(&fixed), ["c", "b", "a"]);
        assert_eq!(fixed.current(), Some(2), "the same tab stays current");
    }

    #[test]
    fn groups_keep_their_tabs_together() {
        let mut l = list(&["a", "b", "c", "d", "e"]);
        let (g, at) = l.create_group(1, " Research ", "#ff0000").unwrap();
        assert_eq!(at, 1);
        assert_eq!(l.group(g).unwrap().name, "Research");
        // Joining from the right end moves the tab after the group's last tab.
        assert_eq!(l.group_target(4, Some(g)), Some(2));
        assert_eq!(l.set_group(4, Some(g)), Some(2));
        assert_eq!(urls(&l), ["a", "b", "e", "c", "d"]);
        // Joining from the left too.
        assert_eq!(l.set_group(0, Some(g)), Some(2));
        assert_eq!(urls(&l), ["b", "e", "a", "c", "d"]);
        assert_eq!(groups_of(&l), [Some(g), Some(g), Some(g), None, None]);
        // Leaving from the middle moves the tab out after the group.
        assert_eq!(l.set_group(1, None), Some(2));
        assert_eq!(urls(&l), ["b", "a", "e", "c", "d"]);
        assert_eq!(groups_of(&l), [Some(g), Some(g), None, None, None]);
        // A tab dropped inside a group joins it; dragged out, it leaves.
        assert!(l.move_tab(3, 1));
        assert_eq!(groups_of(&l)[1], Some(g));
        assert!(l.move_tab(1, 4));
        assert_eq!(groups_of(&l), [Some(g), Some(g), None, None, None]);
        // Pinning takes a tab out of its group; the last one dissolves it.
        l.pin(0, Pin::Workspace);
        l.pin(1, Pin::Workspace);
        assert!(l.group(g).is_none());
        assert!(l.groups().is_empty());
    }

    #[test]
    fn dragging_across_a_group_passes_it() {
        let mut l = list(&["a", "b", "c", "d"]);
        let (g, _) = l.create_group(1, "", "").unwrap();
        l.set_group(2, Some(g));
        // Drag "a" right step by step, past both group tabs.
        assert!(l.drag_tab(0, 1));
        assert!(l.drag_tab(1, 2));
        assert!(l.drag_tab(2, 3));
        assert!(!l.drop_tab(3));
        assert_eq!(urls(&l), ["b", "c", "d", "a"]);
        assert_eq!(groups_of(&l), [Some(g), Some(g), None, None]);
        // A single-tab group survives being moved on its own.
        let (h, at) = l.create_group(3, "", "").unwrap();
        assert!(l.move_tab(at, 2));
        assert_eq!(l.group_rows(h), [2]);
    }

    #[test]
    fn collapsing_the_current_group_moves_off_it_and_activating_unfolds() {
        let mut l = list(&["a", "b", "c"]);
        let (g, _) = l.create_group(0, "", "").unwrap();
        l.set_group(1, Some(g));
        l.activate(1);
        let rev = l.groups_revision();
        assert!(l.set_group_collapsed(g, true));
        assert_eq!(l.current(), Some(2));
        assert!(l.group(g).unwrap().collapsed);
        assert!(l.groups_revision() > rev);
        assert!(l.activate(0));
        assert!(!l.group(g).unwrap().collapsed);
        // With nothing outside the group, it stays open.
        let mut solo = list(&["a"]);
        let (s, _) = solo.create_group(0, "", "").unwrap();
        assert!(!solo.set_group_collapsed(s, true));
    }

    #[test]
    fn links_from_a_grouped_tab_join_its_group() {
        let mut l = list(&["a", "b", "c"]);
        let (g, _) = l.create_group(0, "", "").unwrap();
        l.set_group(1, Some(g));
        assert_eq!(l.next_to_current_index(), 1);
        assert_eq!(l.open_next_to_current("x", false), 1);
        assert_eq!(groups_of(&l), [Some(g), Some(g), Some(g), None]);
        l.activate(3);
        assert_eq!(l.open_next_to_current("y", false), 4);
        assert_eq!(l.get(4).unwrap().group, None);
    }

    #[test]
    fn groups_round_trip_and_stray_members_are_split_off() {
        let mut l = list(&["a", "b", "c"]);
        let (g, _) = l.create_group(0, "G", "#00ff00").unwrap();
        l.set_group(1, Some(g));
        l.set_group_collapsed(g, true);
        let session = l.to_session();
        let back = TabList::from_session(&Session::from_json(&session.to_json()).unwrap());
        assert_eq!(back.groups(), l.groups());
        assert_eq!(groups_of(&back), groups_of(&l));
        // a, b in g, c not; a hand-edited c in g too, but apart: kept apart.
        let mut odd = session.clone();
        odd.tabs.swap(1, 2);
        odd.tabs[2].group = Some(g);
        let fixed = TabList::from_session(&odd);
        assert_eq!(groups_of(&fixed), [Some(g), None, None]);
        // Groups nobody uses, or unknown ids, are dropped.
        let mut unknown = session.clone();
        unknown.tabs[0].group = Some(77);
        unknown.tabs[1].group = None;
        assert!(TabList::from_session(&unknown).groups().is_empty());
    }

    #[test]
    fn closing_a_group_tab_and_reopening_it_rejoins_the_group() {
        let mut l = list(&["a", "b", "c"]);
        let (g, _) = l.create_group(0, "", "").unwrap();
        l.set_group(1, Some(g));
        l.set_group(2, Some(g));
        l.close(1);
        assert_eq!(l.reopen_closed(), Some(1));
        assert_eq!(groups_of(&l), [Some(g), Some(g), Some(g)]);
        // Reopening after the group is gone opens it ungrouped.
        l.close(0);
        l.ungroup(g);
        assert_eq!(l.reopen_closed(), Some(0));
        assert_eq!(l.get(0).unwrap().group, None);
    }

    #[test]
    fn any_recently_closed_tab_can_be_reopened() {
        let mut l = list(&["a", "b", "c", "d"]);
        l.close(3);
        l.close(1);
        l.close(0);
        let closed: Vec<_> = l.closed_tabs().map(|c| c.tab.url.as_str()).collect();
        assert_eq!(closed, ["a", "b", "d"]);
        assert_eq!(l.reopen_index_at(2), Some(1));
        assert_eq!(l.reopen_closed_at(2), Some(1));
        assert_eq!(urls(&l), ["c", "d"]);
        assert_eq!(l.reopen_closed_at(5), None);
        // A closed pinned tab comes back pinned, in front.
        l.pin(1, Pin::Workspace);
        l.close(0);
        assert_eq!(l.reopen_index_at(0), Some(0));
        assert_eq!(l.reopen_closed(), Some(0));
        assert!(l.get(0).unwrap().pinned);
    }

    #[test]
    fn moving_to_a_workspace_goes_to_the_end_and_leaves_the_group() {
        let mut l = list(&["a", "b", "c"]);
        let work = l.add_workspace("Work", "");
        let (g, _) = l.create_group(0, "", "").unwrap();
        l.set_group(1, Some(g));
        assert_eq!(l.move_to_workspace(0, work), Some(2));
        assert_eq!(urls(&l), ["b", "c", "a"]);
        assert_eq!(l.get(2).unwrap().group, None);
        assert_eq!(l.rows_in(work), [2]);
    }

    /// A stand-in for the palette's fuzzy matcher: substring, longer is better.
    fn contains(query: &'static str) -> impl Fn(&str) -> Option<i32> {
        move |text: &str| {
            text.to_lowercase()
                .contains(query)
                .then_some(query.len() as i32 * 2)
        }
    }

    #[test]
    fn search_finds_tabs_in_every_workspace_most_recent_first() {
        let mut l = list(&[
            "https://news.example/",
            "https://mail.example/",
            "https://docs.example/",
        ]);
        l.set_title(2, "Mail drafts");
        let work = l.add_workspace("Work", "");
        l.open_in(work, 9, "https://mail.work.example/", "", false);
        l.note_current(10);
        l.activate(1);
        l.note_current(20);
        l.activate(2);
        l.note_current(30);
        // Title beats address; then most recent.
        assert_eq!(l.search(contains("mail")), [2, 1, 3]);
        // Blank query: everything, current first.
        assert_eq!(l.search(|_| Some(0))[0], 2);
        assert_eq!(l.search(|_| Some(0)).len(), 4);
        assert!(l.search(contains("zzz")).is_empty());
        // The scheme and www. are not matched.
        assert!(l.search(contains("https")).is_empty());
    }

    #[test]
    fn closed_tabs_can_be_searched() {
        let mut l = list(&["a.example/x", "b.example/y", "c"]);
        l.close(0);
        l.close(0);
        assert_eq!(l.search_closed(contains("example")), [0, 1]);
        assert_eq!(l.search_closed(contains("a.ex")), [1]);
    }

    #[test]
    fn split_panes_on_screen_stay_awake() {
        let mut l = list(&["a", "b", "c", "d"]);
        l.note_current(10);
        assert!(l.set_suspended(2, true));
        l.split_tabs(0, 2).unwrap();
        // Joining the split view on screen wakes the tab, and it can't sleep.
        assert_eq!(l.note_current(20), [2]);
        assert!(!l.set_suspended(2, true));
        assert_eq!(l.idle_tabs(1000, 60), [1, 3]);
    }

    #[test]
    fn split_views_hold_two_to_four_tabs() {
        let mut l = list(&["a", "b", "c", "d", "e", "f"]);
        let id = l.split_tabs(0, 2).unwrap();
        assert_eq!(l.split_of(2).unwrap().id, id);
        assert_eq!(
            l.current_split().unwrap().tabs,
            [l.get(0).unwrap().id, l.get(2).unwrap().id]
        );
        // A third tab goes right after the one it splits with.
        l.split_tabs(0, 1);
        let ids = |l: &TabList| -> Vec<TabId> { l.current_split().unwrap().tabs.clone() };
        assert_eq!(ids(&l), [0, 1, 2]);
        assert!((l.current_split().unwrap().sizes[0] - 1.0 / 3.0).abs() < 1e-9);
        l.split_tabs(2, 3);
        assert_eq!(l.split_tabs(0, 4), None, "four panes at most");
        // Splitting with a tab of another split moves it over.
        l.split_tabs(4, 5);
        let other = l.split_tabs(4, 3).unwrap();
        assert_eq!(l.split_of(3).unwrap().id, other);
        assert_eq!(ids(&l), [0, 1, 2]);
        assert_eq!(l.split_tabs(0, 0), Some(id));
    }

    #[test]
    fn closing_or_unsplitting_ends_a_split_left_with_one_tab() {
        let mut l = list(&["a", "b", "c"]);
        l.split_tabs(0, 1);
        l.split_tabs(0, 2);
        let rev = l.splits_revision();
        l.close(1);
        assert!(l.splits_revision() > rev);
        assert_eq!(l.split_of(0).unwrap().tabs.len(), 2);
        assert!(l.unsplit(1));
        assert!(l.splits().is_empty());
        assert!(!l.unsplit(0));
    }

    #[test]
    fn split_sizes_stay_sane() {
        let mut l = list(&["a", "b"]);
        let id = l.split_tabs(0, 1).unwrap();
        assert!(l.set_split_sizes(id, &[3.0, 1.0]));
        assert_eq!(l.splits()[0].sizes, [0.75, 0.25]);
        assert!(!l.set_split_sizes(id, &[3.0, 1.0]));
        assert!(!l.set_split_sizes(id, &[1.0]));
        l.set_split_sizes(id, &[1.0, 0.0]);
        assert_eq!(l.splits()[0].sizes, [0.5, 0.5], "unusable sizes even out");
        l.set_split_sizes(id, &[0.99, 0.01]);
        assert!(l.splits()[0].sizes[1] >= SPLIT_MIN_SIZE * 0.9);
        assert!(l.set_split_vertical(id, true));
        assert!(l.move_pane(0, 1));
        assert_eq!(l.splits()[0].tabs, [1, 0]);
        assert!(!l.move_pane(0, 1));
        assert!(l.end_split(id));
        assert!(l.current_split().is_none());
    }

    #[test]
    fn splits_survive_a_session_round_trip() {
        let mut l = list(&["a", "b", "c"]);
        let id = l.split_tabs(2, 0).unwrap();
        l.set_split_sizes(id, &[0.6, 0.4]);
        l.set_split_vertical(id, true);
        let session = Session::from_json(&l.to_session().to_json()).unwrap();
        assert_eq!(session.splits[0].tabs, [2, 0]);
        let back = TabList::from_session(&session);
        let split = back.split_of(0).unwrap();
        assert_eq!(
            split.tabs,
            [back.get(2).unwrap().id, back.get(0).unwrap().id]
        );
        assert!(split.vertical);
        assert!((split.sizes[0] - 0.6).abs() < 1e-9);
        // Out of range or repeated tabs are dropped.
        let mut odd = session.clone();
        odd.splits[0].tabs = vec![1, 9];
        assert!(TabList::from_session(&odd).splits().is_empty());
        odd.splits[0].tabs = vec![1, 1];
        assert!(TabList::from_session(&odd).splits().is_empty());
    }
}
