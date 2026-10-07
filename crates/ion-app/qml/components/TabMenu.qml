import QtQuick
import QtQuick.Controls
import Ion

// Right-click menu for one tab in either strip. `browser` is the main window,
// for its tab helpers.
Menu {
    id: menu

    required property var browser
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
        popup()
    }

    MenuItem {
        text: menu.browser.verticalTabs ? qsTr("New tab below") : qsTr("New tab to the right")
        onTriggered: {
            Tabs.openTabAt(menu.index + 1, "", true)
            menu.browser.focusUrlBar()
        }
    }

    MenuSeparator {}

    MenuItem {
        text: qsTr("Reload")
        onTriggered: menu.view?.reload()
    }
    MenuItem {
        text: qsTr("Duplicate")
        onTriggered: Tabs.openTabAt(menu.index + 1, Tabs.urlAt(menu.index), true)
    }
    MenuItem {
        text: menu.view?.audioMuted ? qsTr("Unmute tab") : qsTr("Mute tab")
        onTriggered: {
            if (menu.view)
                menu.view.audioMuted = !menu.view.audioMuted
        }
    }
    MenuItem {
        text: qsTr("Suspend")
        // The current tab is always awake; see general.suspendTabsAfter.
        enabled: !menu.isCurrent && !(menu.view?.suspended ?? true)
        onTriggered: Tabs.setSuspended(menu.index, true)
    }

    MenuSeparator {}

    MenuItem {
        text: qsTr("Close tab")
        onTriggered: menu.browser.closeTab(menu.index)
    }
    MenuItem {
        text: qsTr("Close other tabs")
        enabled: Tabs.count > 1
        onTriggered: menu.browser.closeOtherTabs(menu.index)
    }
    MenuItem {
        text: menu.browser.verticalTabs ? qsTr("Close tabs below") : qsTr("Close tabs to the right")
        enabled: menu.index < Tabs.count - 1
        onTriggered: menu.browser.closeTabsAfter(menu.index)
    }
    MenuItem {
        text: qsTr("Reopen closed tab")
        enabled: Tabs.closedCount > 0
        onTriggered: Tabs.reopenClosedTab()
    }
}
