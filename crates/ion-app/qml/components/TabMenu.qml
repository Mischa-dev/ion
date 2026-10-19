import QtQuick
import QtQuick.Controls
import Ion

// Right-click menu for one tab in either strip. `browser` is the main window,
// for its tab helpers.
IonMenu {
    // Not `menu`: inside a MenuItem that name is the item's own (sub)menu.
    id: root

    required property var browser
    // For "New workspace…" from the move submenu.
    required property var workspaceMenu
    // For naming a new tab group.
    required property var groupMenu
    // This workspace's groups as `{id, name, color, collapsed, tabs}`, and the
    // tab's group and pin (0 none, 1 workspace, 2 Essential), refreshed when
    // the menu opens.
    property var groups: []
    property int group: -1
    property int pin: 0
    // The split view showing the tab, as `{id, tabs, sizes, vertical}`, or
    // null; and whether the current tab's split has room for one more.
    property var split: null
    property bool splitHasRoom: true
    // Workspaces as `{id, name, color, tabs}`, refreshed when the menu opens.
    property var workspaces: []
    // The tab the menu was opened on, and its web view.
    property int index: -1
    property var view: null
    readonly property bool isCurrent: index === Tabs.currentIndex

    palette.window: Theme.surface
    palette.windowText: Theme.text
    palette.base: Theme.surfaceRaised
    palette.text: Theme.text
    palette.button: Theme.surfaceRaised
    palette.buttonText: Theme.text
    palette.light: Theme.surfaceHover
    palette.midlight: Theme.surfaceHover
    palette.mid: Theme.border
    palette.dark: Theme.border
    palette.highlight: Theme.accent
    palette.highlightedText: Theme.onAccent

    function openFor(tabIndex) {
        index = tabIndex
        view = browser.viewAt(tabIndex)
        workspaces = JSON.parse(Tabs.workspaces())
        groups = JSON.parse(Tabs.groups(Tabs.workspace))
        group = Tabs.groupAt(tabIndex)
        pin = Tabs.pinnedAt(tabIndex)
        const current = JSON.parse(Tabs.currentSplit())
        split = current && current.tabs.includes(tabIndex) ? current : null
        splitHasRoom = !current || current.tabs.length < 4
        popup()
    }

    IonMenuItem {
        text: root.browser.verticalTabs ? qsTr("New tab below") : qsTr("New tab to the right")
        onTriggered: {
            Tabs.openTabAt(root.index + 1, "", true)
            root.browser.focusUrlBar()
        }
    }

    IonMenuSeparator {}

    IonMenuItem {
        text: qsTr("Reload")
        onTriggered: root.view?.reload()
    }
    IonMenuItem {
        text: qsTr("Duplicate")
        onTriggered: Tabs.openTabAt(root.index + 1, Tabs.urlAt(root.index), true)
    }
    IonMenuItem {
        text: root.view?.audioMuted ? qsTr("Unmute tab") : qsTr("Mute tab")
        onTriggered: {
            if (root.view)
                root.view.audioMuted = !root.view.audioMuted
        }
    }
    IonMenuItem {
        text: qsTr("Put to sleep")
        // Tabs on screen (the current one and its split's panes) stay awake;
        // see general.suspendTabsAfter. Their views are the visible ones. So
        // do tabs holding a dialog for when they are shown.
        enabled: !(root.view?.visible ?? true) && !(root.view?.suspended ?? true)
                 && !(root.view?.needsAttention ?? false)
        onTriggered: Tabs.setSuspended(root.index, true)
    }

    IonMenuSeparator {}

    IonMenuItem {
        text: qsTr("Open in split view")
        visible: !root.isCurrent && !root.split && root.splitHasRoom && root.pin !== 2
                 && Tabs.pinnedAt(Tabs.currentIndex) !== 2
        height: visible ? implicitHeight : 0
        onTriggered: Tabs.splitTabs(Tabs.currentIndex, root.index)
    }
    IonMenuItem {
        text: qsTr("Split with a new tab")
        visible: root.isCurrent && root.splitHasRoom && root.pin !== 2
        height: visible ? implicitHeight : 0
        onTriggered: {
            const index = Tabs.openTabNextToCurrent("", false)
            Tabs.splitTabs(root.index, index)
            Tabs.activate(index)
            root.browser.focusUrlBar()
        }
    }
    IonMenuItem {
        text: root.split?.vertical ? qsTr("Panes side by side") : qsTr("Stack panes")
        visible: root.split !== null
        height: visible ? implicitHeight : 0
        onTriggered: Tabs.setSplitVertical(root.split.id, !root.split.vertical)
    }
    IonMenuItem {
        text: qsTr("Remove from split view")
        visible: root.split !== null
        height: visible ? implicitHeight : 0
        onTriggered: Tabs.unsplit(root.index)
    }
    IonMenuItem {
        text: qsTr("End split view")
        visible: root.split !== null
        height: visible ? implicitHeight : 0
        onTriggered: Tabs.endSplit(root.split.id)
    }

    IonMenuSeparator {}

    IonMenuItem {
        text: root.pin === 1 ? qsTr("Unpin tab") : qsTr("Pin tab")
        onTriggered: Tabs.pinTab(root.index, root.pin === 1 ? 0 : 1)
    }
    IonMenuItem {
        text: root.pin === 2 ? qsTr("Remove from Essentials") : qsTr("Add to Essentials")
        onTriggered: Tabs.pinTab(root.index, root.pin === 2 ? 0 : 2)
    }
    IonMenuItem {
        text: qsTr("Add to new group")
        visible: root.pin === 0
        height: visible ? implicitHeight : 0
        onTriggered: root.groupMenu.create(root.index)
    }
    IonMenu {
        id: groupsMenu
        palette: root.palette
        title: qsTr("Add to group")
        enabled: root.pin === 0 && root.groups.some(g => g.id !== root.group)

        Instantiator {
            model: root.groups.filter(g => g.id !== root.group)
            delegate: IonMenuItem {
                required property var modelData
                text: modelData.name.length > 0 ? modelData.name : qsTr("Unnamed group")
                onTriggered: Tabs.setGroup(root.index, modelData.id)
            }
            onObjectAdded: (index, object) => groupsMenu.insertItem(index, object)
            onObjectRemoved: (index, object) => groupsMenu.removeItem(object)
        }
    }
    IonMenuItem {
        text: qsTr("Remove from group")
        visible: root.group >= 0
        height: visible ? implicitHeight : 0
        onTriggered: Tabs.setGroup(root.index, -1)
    }

    IonMenu {
        id: moveMenu
        palette: root.palette
        title: qsTr("Move to workspace")
        enabled: root.pin !== 2

        Instantiator {
            model: root.workspaces.filter(w => w.id !== Tabs.workspaceAt(root.index))
            delegate: IonMenuItem {
                required property var modelData
                text: modelData.name
                onTriggered: Tabs.moveToWorkspace(root.index, modelData.id)
            }
            onObjectAdded: (index, object) => moveMenu.insertItem(index, object)
            onObjectRemoved: (index, object) => moveMenu.removeItem(object)
        }

        IonMenuSeparator {}

        IonMenuItem {
            text: qsTr("New workspace…")
            onTriggered: root.workspaceMenu.create(root.index)
        }
    }

    IonMenuSeparator {}

    IonMenuItem {
        text: qsTr("Close tab")
        onTriggered: root.browser.closeTab(root.index)
    }
    IonMenuItem {
        text: qsTr("Close other tabs")
        enabled: {
            root.workspaces // re-read each time the menu opens
            return Tabs.neighbour(root.index, 1) >= 0 || Tabs.neighbour(root.index, -1) >= 0
        }
        onTriggered: root.browser.closeOtherTabs(root.index)
    }
    IonMenuItem {
        text: root.browser.verticalTabs ? qsTr("Close tabs below") : qsTr("Close tabs to the right")
        enabled: {
            root.workspaces
            return Tabs.neighbour(root.index, 1) >= 0
        }
        onTriggered: root.browser.closeTabsAfter(root.index)
    }
    IonMenuItem {
        text: qsTr("Reopen closed tab")
        enabled: Tabs.closedCount > 0
        onTriggered: Tabs.reopenClosedTab()
    }
}
