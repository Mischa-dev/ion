# Tabs

How Ion arranges tabs, and where each piece lives. The logic is in
`crates/ion-session/src/tabs.rs` (`TabList`, unit tested); `bridge/tabs.rs`
exposes it to QML as the `Tabs` model; the strips are
`qml/components/TabStrip.qml` (horizontal) and `VerticalTabStrip.qml`
(sidebar).

## Order

A strip shows, left to right (top to bottom in the sidebar):

1. **Essentials**: tabs pinned in every workspace. Icon tiles; in the sidebar
   a grid at the top. Showing one keeps the current workspace's strip.
2. **Pinned tabs** of the workspace: icons in the horizontal strip, rows above
   a divider in the sidebar.
3. The workspace's other tabs, with **tab groups** among them.

New tabs never land among Essentials or pinned tabs, and dragging keeps a tab
within its section. "Close other tabs" and "Close tabs to the right" leave
pinned tabs and Essentials open. Pin, unpin and add to Essentials from the
tab's right-click menu.

## Tab groups

A group is a run of neighbouring tabs with a color and an optional name.
"Add to new group" in the tab menu starts one and asks for a name. The group's
header (a chip in the strip, a row in the sidebar) folds and unfolds it;
right-click it to rename, recolor, add a tab, ungroup or close the group.

- Links opened from a grouped tab join its group.
- Dragging a tab shows where it will land; letting go inside a group adds it,
  letting go outside
  takes it out. Dragging past a group does not.
- Activating a tab in a folded group unfolds it. Folding the current tab's
  group moves to the nearest tab outside it.
- Groups belong to one workspace; moving a tab to another workspace takes it
  out of its group. Pinned tabs are never grouped.

Groups, pins and Essentials are saved with the session and with named
sessions.

## Tab search

Ctrl/Cmd+Shift+A (or the ⌕ button in either strip, or "Search tabs…" in the
⋯ menu) lists every open tab in every workspace, most recently used first,
then recently closed tabs. Typing narrows both by title or address. Enter
switches to the tab, and to its workspace, or reopens the closed tab; the ×
on a row closes that tab. With an empty search, Enter goes back to the tab
used before the current one.

The ⋯ menu's "Recently closed" lists the last ten closed tabs; any of them
can be reopened, not only the latest (Ctrl/Cmd+Shift+T).

## Workspaces

Each workspace keeps its own tabs, pinned tabs and groups; Essentials are
shared by all of them. With more than one workspace, the sidebar's foot
shows a dot (or icon) per workspace next to the current one's name:

- Click a dot to switch, or press Ctrl/Cmd+Alt+PgUp / PgDown.
- Swipe sideways on the tab list with a touchpad to go to the next or
  previous workspace, one per swipe.
- Drop a dragged tab on a dot to move it to that workspace; the page stays
  where it was.

The workspace button opens the menu to add, rename, recolor, reorder or
delete workspaces.

## Hover previews

Resting the pointer on a tab shows a card with its full title, its site and
a picture of the page from when it was last on screen, below the tab in the
top strip and beside it in the sidebar. The card also says when a tab is
asleep, playing or muted, or waiting for you.

A page in a background tab that opens a dialog (alert, confirm, a sign-in
prompt) doesn't interrupt the tab in front: the dialog waits until its tab
is shown, and the tab is marked with a pulsing dot meanwhile.

## Split view

Up to four tabs can share the page area, side by side or stacked:

- Drag a tab from either strip onto the page; the half it would take
  lights up, and letting go opens it there next to the page shown.
- Ctrl/Cmd+Alt+S splits the current tab with the tab used before it (or a
  new tab), and ends the split when pressed again.
- A tab's menu has "Open in split view", "Split with a new tab", "Stack
  panes", "Remove from split view" and "End split view".

Clicking into a pane makes its tab the current one, outlined in the accent
color; the toolbar and shortcuts act on it. Drag the bar between panes to
resize them. Tabs in a split view are marked ◫ in the strips, and choosing
any of them brings the whole split back. Splits are saved with the session;
closing a tab takes it out of its split, and a split left with one tab ends.
