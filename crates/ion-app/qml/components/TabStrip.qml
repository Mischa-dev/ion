import QtQuick
import QtQuick.Controls
import QtQml.Models
import QtQuick.Layouts
import Ion

// Horizontal tab strip over the `Tabs` model. Reports clicks; Main.qml decides
// what they do. VerticalTabStrip.qml is the sidebar variant.
//
// Essentials come first, then this workspace's pinned tabs as icons, then
// its other tabs. A tab group shows as a colored chip before its tabs, with
// a line under them; clicking the chip folds the group to just the chip.
Rectangle {
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
    // Tab counts by group id, for folded group chips, and the space they and
    // the pinned tabs take, which the other tabs share what is left of.
    property var groupTabs: ({})
    property int fixedWidth: 0
    property int flexibleTabs: 1

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

    function viewAt(row) {
        strip.viewsRevision
        return strip.views ? strip.views.itemAt(row) : null
    }

    function refreshGroups() {
        const counts = {}
        const groups = JSON.parse(Tabs.groups(strip.workspace))
        for (const g of groups)
            counts[g.id] = g.tabs
        groupTabs = counts
        const layout = JSON.parse(Tabs.stripLayout(strip.workspace))
        flexibleTabs = Math.max(1, layout.tabs)
        fixedWidth = layout.pinned * (Theme.tabHeight + Theme.spacing / 2) + Theme.spacing * 2
                   + groups.length * Theme.tabMaxWidth / 2
    }
    onWorkspaceChanged: refreshGroups()
    Component.onCompleted: refreshGroups()
    Connections {
        target: Tabs
        function onGroupsChanged() { strip.refreshGroups() }
        function onSessionChanged() { strip.refreshGroups() }
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

    implicitHeight: Theme.tabHeight + Theme.spacing * 2
    color: Theme.background

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        anchors.topMargin: Theme.spacing
        spacing: Theme.spacing / 2

        WorkspaceButton {
            Layout.alignment: Qt.AlignVCenter
            Layout.maximumWidth: Theme.tabMaxWidth / 2
            onMenuRequested: anchor => strip.workspaceMenuRequested(anchor)
        }

        // Essentials, shared by every workspace.
        ListView {
            id: essentialsList
            Layout.fillHeight: true
            Layout.preferredWidth: contentWidth
            visible: count > 0
            orientation: ListView.Horizontal
            spacing: Theme.spacing / 2
            interactive: false
            model: SortFilterProxyModel {
                model: strip.tabs
                filters: ValueFilter { roleName: "essential"; value: true }
            }
            delegate: PinnedTab {
                width: Theme.tabHeight
                height: essentialsList.height
                list: essentialsList
                currentIndex: strip.currentIndex
                view: strip.viewAt(row)
                onActivated: index => strip.activated(index)
                onCloseRequested: index => strip.closeRequested(index)
                onMoveRequested: (from, to) => strip.moveRequested(from, to)
                onMenuRequested: index => strip.tabMenuRequested(index)
                onDraggedOut: (index, pos) => strip.tabDraggedOut(index, pos)
                onDroppedOut: (index, pos) => strip.tabDroppedOut(index, pos)
            }
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: ListView.Horizontal
            // Gaps are inside the delegates, so folded tabs leave none.
            spacing: 0
            clip: true
            interactive: contentWidth > width
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
                // Pinned tabs end with a wider gap before the other tabs.
                readonly property real trailing: pinned && sectionEnd ? Theme.spacing * 2 : gap
                readonly property real tabWidth: pinned ? Theme.tabHeight
                    : Math.min(Theme.tabMaxWidth, Math.max(Theme.tabMinWidth, (list.width - strip.fixedWidth) / strip.flexibleTabs - gap))

                width: (groupFirst ? chip.implicitWidth + gap : 0) + (folded ? 0 : tabWidth + trailing)
                height: list.height
                visible: width > 0
                z: body.z

                GroupChip {
                    id: chip
                    visible: slot.groupFirst
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    group: slot.group
                    name: slot.groupName
                    groupColor: slot.groupColor
                    collapsed: slot.groupCollapsed
                    tabs: strip.groupTabs[slot.group] ?? 0
                    onToggled: Tabs.setGroupCollapsed(slot.group, !slot.groupCollapsed)
                    onMenuRequested: anchor => strip.groupMenuRequested(slot.group, anchor)
                }

                Loader {
                    id: body
                    x: slot.groupFirst ? chip.implicitWidth + slot.gap : 0
                    // Inset from the clipped list edges, leaving room for the
                    // current tab's shadow.
                    y: Theme.spacing / 2
                    width: slot.tabWidth
                    height: parent.height - Theme.spacing
                    visible: !slot.folded
                    // Kept loaded while folded: unloading would end a drag passing by.
                    active: true
                    z: item?.z ?? 0
                    sourceComponent: slot.pinned ? pinnedTab : fullTab
                }

                // Ties a group's tabs together, through the gaps between them.
                Rectangle {
                    visible: slot.grouped && !slot.folded
                    x: body.x - (slot.groupFirst ? slot.gap : 0)
                    width: body.width + (slot.groupFirst ? slot.gap : 0) + (slot.groupLast ? 0 : slot.gap)
                    anchors.bottom: parent.bottom
                    height: Theme.groupLineWidth
                    radius: height / 2
                    color: slot.groupColor.length > 0 ? slot.groupColor : Theme.accent
                }

                Component {
                    id: fullTab
                    TabRow {
                        row: slot.row
                        title: slot.title
                        url: slot.url
                        icon: slot.icon
                        suspended: slot.suspended
                        // Not `list: list`: inside the tab that names its own property.
                        list: slot.ListView.view
                        currentIndex: strip.currentIndex
                        view: strip.viewAt(slot.row)
                        inSplit: slot.split >= 0
                        splitShowing: slot.split >= 0 && slot.split === strip.currentSplit
                        onActivated: index => strip.activated(index)
                        onCloseRequested: index => strip.closeRequested(index)
                        onMoveRequested: (from, to) => strip.moveRequested(from, to)
                        onMenuRequested: index => strip.tabMenuRequested(index)
                        onDraggedOut: (index, pos) => strip.tabDraggedOut(index, pos)
                        onDroppedOut: (index, pos) => strip.tabDroppedOut(index, pos)
                    }
                }
                Component {
                    id: pinnedTab
                    PinnedTab {
                        row: slot.row
                        title: slot.title
                        url: slot.url
                        icon: slot.icon
                        suspended: slot.suspended
                        essential: false
                        // Not `list: list`: inside the tab that names its own property.
                        list: slot.ListView.view
                        currentIndex: strip.currentIndex
                        view: strip.viewAt(slot.row)
                        onActivated: index => strip.activated(index)
                        onCloseRequested: index => strip.closeRequested(index)
                        onMoveRequested: (from, to) => strip.moveRequested(from, to)
                        onMenuRequested: index => strip.tabMenuRequested(index)
                        onDraggedOut: (index, pos) => strip.tabDraggedOut(index, pos)
                        onDroppedOut: (index, pos) => strip.tabDroppedOut(index, pos)
                    }
                }
            }
        }

        IconButton {
            Layout.alignment: Qt.AlignVCenter
            iconName: "plus"
            tip: qsTr("New tab")
            onClicked: strip.newTabRequested()
        }

        IconButton {
            Layout.alignment: Qt.AlignVCenter
            iconName: "search"
            tip: qsTr("Search tabs")
            onClicked: strip.searchRequested()
        }

        IconButton {
            id: menuButton
            Layout.alignment: Qt.AlignVCenter
            iconName: "more"
            tip: qsTr("Tabs and sessions")
            onClicked: strip.menuRequested(menuButton)
        }
    }
}
