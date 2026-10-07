import QtQuick
import QtQuick.Controls
import Ion

// Right-click menu for one tab in either strip. `browser` is the main window,
// for its tab helpers.
Menu {
    // Not `menu`: inside a MenuItem that name is the item's own (sub)menu.
    id: root

    required property var browser
    // For "New workspace…" from the move submenu.
    required property var workspaceMenu
    // Workspaces as `{id, name, color, icon, tabs}`, refreshed when the menu opens.
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
        popup()
    }

    MenuItem {
        text: root.browser.verticalTabs ? qsTr("New tab below") : qsTr("New tab to the right")
        onTriggered: {
            Tabs.openTabAt(root.index + 1, "", true)
            root.browser.focusUrlBar()
        }
    }

    MenuSeparator {}

    MenuItem {
        text: qsTr("Reload")
        onTriggered: root.view?.reload()
    }
    MenuItem {
        text: qsTr("Duplicate")
        onTriggered: Tabs.openTabAt(root.index + 1, Tabs.urlAt(root.index), true)
    }
    MenuItem {
        text: root.view?.audioMuted ? qsTr("Unmute tab") : qsTr("Mute tab")
        onTriggered: {
            if (root.view)
                root.view.audioMuted = !root.view.audioMuted
        }
    }
    MenuItem {
        text: qsTr("Suspend")
        // The current tab is always awake; see general.suspendTabsAfter.
        enabled: !root.isCurrent && !(root.view?.suspended ?? true)
        onTriggered: Tabs.setSuspended(root.index, true)
    }

    Menu {
        id: moveMenu
        palette: root.palette
        title: qsTr("Move to workspace")

        Instantiator {
            model: root.workspaces.filter(w => w.id !== Tabs.workspaceAt(root.index))
            delegate: MenuItem {
                id: item
                required property var modelData
                text: modelData.name
                onTriggered: Tabs.moveToWorkspace(root.index, modelData.id)

                // Plain text: the stock label would render markup in names.
                contentItem: Text {
                    text: item.text
                    textFormat: Text.PlainText
                    font: item.font
                    color: item.highlighted ? item.palette.highlightedText : item.palette.windowText
                    elide: Text.ElideRight
                    verticalAlignment: Text.AlignVCenter
                }
            }
            onObjectAdded: (index, object) => moveMenu.insertItem(index, object)
            onObjectRemoved: (index, object) => moveMenu.removeItem(object)
        }

        MenuSeparator {}

        MenuItem {
            text: qsTr("New workspace…")
            onTriggered: root.workspaceMenu.create(root.index)
        }
    }

    MenuSeparator {}

    MenuItem {
        text: qsTr("Close tab")
        onTriggered: root.browser.closeTab(root.index)
    }
    MenuItem {
        text: qsTr("Close other tabs")
        enabled: {
            root.workspaces // re-read each time the menu opens
            return Tabs.workspaceTabCount(Tabs.workspaceAt(root.index)) > 1
        }
        onTriggered: root.browser.closeOtherTabs(root.index)
    }
    MenuItem {
        text: root.browser.verticalTabs ? qsTr("Close tabs below") : qsTr("Close tabs to the right")
        enabled: {
            root.workspaces
            return Tabs.neighbour(root.index, 1) >= 0
        }
        onTriggered: root.browser.closeTabsAfter(root.index)
    }
    MenuItem {
        text: qsTr("Reopen closed tab")
        enabled: Tabs.closedCount > 0
        onTriggered: Tabs.reopenClosedTab()
    }
}
