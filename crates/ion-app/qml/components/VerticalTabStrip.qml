import QtQuick
import QtQuick.Controls
import QtQml.Models
import QtQuick.Layouts
import Ion

// Vertical variant of TabStrip: a sidebar listing tabs top to bottom, for
// people who keep many tabs open. Same model, same signals.
//
// Essentials sit in a grid of tiles at the top, then this workspace's pinned
// tabs above a divider, then its other tabs. A tab group has a header row
// that folds it; its tabs are indented along a line in its color.
//
// Collapsed (`ui.collapseSidebar`), it shows only favicons and expands over the
// page while the pointer rests on it.
Item {
    id: strip

    required property var tabs   // the Tabs model
    property int currentIndex: 0
    // Only this workspace's tabs are listed (`Tabs.workspace`).
    property int workspace: 0
    // The Repeater of BrowserTabs in Main.qml, for favicons and load state, and
    // a counter bumped whenever it gains or loses a view.
    property var views: null
    property int viewsRevision: 0
    // Id of the split view on screen, or -1.
    property int currentSplit: -1

    signal activated(int index)
    signal closeRequested(int index)
    signal moveRequested(int from, int to)
    // A tab dragged off the strip, at `scenePos` in window coordinates;
    // index -1 when it came back or the drag ended.
    signal tabDraggedOut(int index, point scenePos)
    signal tabDroppedOut(int index, point scenePos)
    signal newTabRequested()
    signal searchRequested()
    signal menuRequested(Item anchor)
    signal tabMenuRequested(int index)
    signal groupMenuRequested(int group, Item anchor)
    signal workspaceMenuRequested(Item anchor)

    // Tab counts by group id, for folded group headers.
    property var groupTabs: ({})

    function viewAt(row) {
        strip.viewsRevision
        return strip.views ? strip.views.itemAt(row) : null
    }

    function refreshGroups() {
        const counts = {}
        for (const g of JSON.parse(Tabs.groups(strip.workspace)))
            counts[g.id] = g.tabs
        groupTabs = counts
    }
    onWorkspaceChanged: refreshGroups()
    Component.onCompleted: refreshGroups()
    Connections {
        target: Tabs
        function onGroupsChanged() { strip.refreshGroups() }
        function onCountChanged() { strip.refreshGroups() }
    }

    // Position in the list of the tab at `row` of the model, or -1. Not bound
    // to the ListView's own currentIndex: the view rewrites that itself as
    // rows come and go, which looped with a binding.
    readonly property int shownCurrent: {
        list.count
        return shown.mapFromSource(strip.tabs.index(strip.currentIndex, 0)).row
    }
    // Keep the current tab fully in view when it changes or tabs are added.
    onShownCurrentChanged: Qt.callLater(() => list.positionViewAtIndex(strip.shownCurrent, ListView.Contain))

    property bool collapsed: false
    // The pointer has rested on the collapsed sidebar.
    property bool peek: false
    // Showing titles: always when not collapsed, else while peeking.
    readonly property bool expanded: !collapsed || peek

    implicitWidth: collapsed ? Theme.sidebarCollapsedWidth : Theme.tabMaxWidth

    // Rest the pointer on a collapsed sidebar to expand it; leave to collapse.
    Timer {
        id: peekTimer
        interval: Theme.hoverDelayMs
        onTriggered: strip.peek = true
    }

    Rectangle {
        id: panel
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        width: strip.expanded ? Theme.tabMaxWidth : Theme.sidebarCollapsedWidth
        clip: true
        color: Theme.background

        Behavior on width { NumberAnimation { duration: Theme.animationMs; easing.type: Easing.OutCubic } }

        HoverHandler {
            onHoveredChanged: {
                if (hovered && strip.collapsed) {
                    peekTimer.restart()
                } else {
                    peekTimer.stop()
                    strip.peek = false
                }
            }
        }

        // Edge between the expanded sidebar and the page it covers.
        Rectangle {
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.right: parent.right
            width: Theme.hairline
            color: Theme.border
            visible: strip.collapsed && strip.peek
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: Theme.spacing
            spacing: Theme.spacing / 2

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spacing / 2

                IconButton {
                    Layout.fillWidth: true
                    glyph: "+"
                    tip: qsTr("New tab")
                    onClicked: strip.newTabRequested()
                }

                IconButton {
                    visible: strip.expanded
                    glyph: "⌕"
                    tip: qsTr("Search tabs")
                    onClicked: strip.searchRequested()
                }

                IconButton {
                    id: menuButton
                    visible: strip.expanded
                    glyph: "⋯"
                    tip: qsTr("Tabs and sessions")
                    onClicked: strip.menuRequested(menuButton)
                }

                IconButton {
                    visible: strip.expanded
                    glyph: strip.collapsed ? "»" : "«"
                    tip: strip.collapsed ? qsTr("Keep sidebar open") : qsTr("Collapse sidebar")
                    onClicked: Config.set("ui.collapseSidebar", !strip.collapsed)
                }
            }

            // Essentials, shared by every workspace, as a grid of tiles.
            GridView {
                id: essentialsGrid
                Layout.fillWidth: true
                Layout.preferredHeight: count > 0 ? contentHeight : 0
                visible: count > 0
                interactive: false
                readonly property int columns: strip.expanded ? Math.max(1, Math.min(count, 4)) : 1
                cellWidth: width / columns
                cellHeight: Theme.tabHeight + Theme.spacing * 2
                model: SortFilterProxyModel {
                    model: strip.tabs
                    filters: ValueFilter { roleName: "essential"; value: true }
                }
                delegate: Item {
                    id: cell
                    required property int row
                    required property string title
                    required property string url
                    required property string icon
                    required property bool suspended
                    width: essentialsGrid.cellWidth
                    height: essentialsGrid.cellHeight

                    PinnedTab {
                        anchors.fill: parent
                        anchors.margins: Theme.spacing / 2
                        row: cell.row
                        title: cell.title
                        url: cell.url
                        icon: cell.icon
                        suspended: cell.suspended
                        essential: true
                        list: essentialsGrid
                        currentIndex: strip.currentIndex
                        view: strip.viewAt(cell.row)
                        previewBeside: true
                        onActivated: index => strip.activated(index)
                        onCloseRequested: index => strip.closeRequested(index)
                        onMoveRequested: (from, to) => strip.moveRequested(from, to)
                        onMenuRequested: index => strip.tabMenuRequested(index)
                        onDraggedOut: (index, pos) => strip.tabDraggedOut(index, pos)
                        onDroppedOut: (index, pos) => strip.tabDroppedOut(index, pos)
                    }
                }
            }

            ListView {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                // Gaps are inside the delegates, so folded tabs leave none.
                spacing: 0
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                // Rows keep their model index in `row`; the list's own indexes
                // only count this workspace's tabs.
                model: SortFilterProxyModel {
                    id: shown
                    model: strip.tabs
                    filters: [
                        ValueFilter { roleName: "workspace"; value: strip.workspace },
                        ValueFilter { roleName: "essential"; value: false }
                    ]
                }
                highlightFollowsCurrentItem: false

                add: Transition {
                    NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationSlowMs; easing.type: Easing.OutCubic }
                }
                remove: Transition {
                    NumberAnimation { property: "opacity"; to: 0; duration: Theme.animationMs }
                }
                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

                delegate: Item {
                    id: slot

                    required property int row
                    required property string title
                    required property string url
                    required property string icon
                    required property bool suspended
                    required property bool pinned
                    required property bool sectionEnd
                    required property int group
                    required property string groupName
                    required property string groupColor
                    required property bool groupCollapsed
                    required property bool groupFirst
                    required property bool groupLast
                    required property int split

                    readonly property bool grouped: group >= 0
                    readonly property bool folded: grouped && groupCollapsed
                    readonly property real gap: Theme.spacing / 2
                    readonly property real headerHeight: groupFirst ? chip.implicitHeight + gap : 0
                    // A line under the last pinned tab sets them apart.
                    readonly property real divider: pinned && sectionEnd ? Theme.spacing * 2 + Theme.hairline : 0
                    readonly property color tint: groupColor.length > 0 ? groupColor : Theme.accent

                    width: list.width
                    height: headerHeight + (folded ? 0 : Theme.tabHeight + gap) + divider
                    visible: height > 0
                    z: tab.z

                    GroupChip {
                        id: chip
                        visible: slot.groupFirst
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        vertical: true
                        compact: !strip.expanded
                        group: slot.group
                        name: slot.groupName
                        groupColor: slot.groupColor
                        collapsed: slot.groupCollapsed
                        tabs: strip.groupTabs[slot.group] ?? 0
                        onToggled: Tabs.setGroupCollapsed(slot.group, !slot.groupCollapsed)
                        onMenuRequested: anchor => strip.groupMenuRequested(slot.group, anchor)
                    }

                    // Ties a group's tabs together down their left edge.
                    Rectangle {
                        visible: slot.grouped && !slot.folded
                        x: Theme.spacing
                        y: slot.headerHeight
                        width: Theme.groupLineWidth
                        height: Theme.tabHeight + (slot.groupLast ? 0 : slot.gap)
                        radius: width / 2
                        color: slot.tint
                    }

                    TabRow {
                        id: tab
                        x: slot.grouped && strip.expanded ? Theme.spacing * 2 : 0
                        y: slot.headerHeight
                        width: parent.width - x
                        height: Theme.tabHeight
                        visible: !slot.folded
                        row: slot.row
                        title: slot.title
                        url: slot.url
                        icon: slot.icon
                        suspended: slot.suspended
                        // Not `list: list`: inside the tab that names its own property.
                        list: slot.ListView.view
                        currentIndex: strip.currentIndex
                        view: strip.viewAt(slot.row)
                        previewBeside: true
                        inSplit: slot.split >= 0
                        splitShowing: slot.split >= 0 && slot.split === strip.currentSplit
                        showTitle: strip.expanded
                        urlAsTitle: true
                        onActivated: index => strip.activated(index)
                        onCloseRequested: index => strip.closeRequested(index)
                        onMoveRequested: (from, to) => strip.moveRequested(from, to)
                        onMenuRequested: index => strip.tabMenuRequested(index)
                        onDraggedOut: (index, pos) => strip.tabDraggedOut(index, pos)
                        onDroppedOut: (index, pos) => strip.tabDroppedOut(index, pos)
                    }

                    Rectangle {
                        visible: slot.divider > 0
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.leftMargin: Theme.spacing
                        anchors.rightMargin: Theme.spacing
                        anchors.bottom: parent.bottom
                        anchors.bottomMargin: Theme.spacing
                        height: Theme.hairline
                        color: Theme.border
                    }
                }
            }

            WorkspaceButton {
                Layout.fillWidth: true
                compact: !strip.expanded
                onMenuRequested: anchor => strip.workspaceMenuRequested(anchor)
            }
        }
    }
}
