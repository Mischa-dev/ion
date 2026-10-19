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
