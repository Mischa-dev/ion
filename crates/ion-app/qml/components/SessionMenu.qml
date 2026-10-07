import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Tab and session actions behind the "⋯" button of the tab strips: reopen a
// closed tab, switch the strip layout, and save, open or delete named sessions.
Item {
    id: root

    // Session names, refreshed every time the menu opens.
    property var names: []

    // Shared by the menus and the dialog (popups don't inherit it from here),
    // so they match the browser chrome.
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
    palette.placeholderText: Theme.textMuted

    function open(anchor) {
        menu.popup(anchor, 0, anchor.height)
    }

    Menu {
        id: menu
        palette: root.palette

        onAboutToShow: root.names = Tabs.sessionNames()

        MenuItem {
            text: qsTr("Reopen closed tab")
            enabled: Tabs.closedCount > 0
            onTriggered: Tabs.reopenClosedTab()
        }
        MenuItem {
            text: qsTr("Vertical tabs")
            checkable: true
            // Writes `ui.tabs` to the local config, like any other setting.
            checked: Config.tabLayout === "vertical"
            onTriggered: Config.set("ui.tabs", checked ? "vertical" : "horizontal")
        }

        MenuSeparator {}

        MenuItem {
            text: qsTr("Save session as…")
            onTriggered: saveDialog.open()
        }

        Menu {
            id: openMenu
            palette: root.palette
            title: qsTr("Open session")
            enabled: root.names.length > 0

            Instantiator {
                model: root.names
                delegate: MenuItem {
                    required property string modelData
                    text: modelData
                    onTriggered: Tabs.openSession(modelData)
                }
                onObjectAdded: (index, object) => openMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => openMenu.removeItem(object)
            }
        }

        Menu {
            id: deleteMenu
            palette: root.palette
            title: qsTr("Delete session")
            enabled: root.names.length > 0

            Instantiator {
                model: root.names
                delegate: MenuItem {
                    required property string modelData
                    text: modelData
                    onTriggered: Tabs.deleteSession(modelData)
                }
                onObjectAdded: (index, object) => deleteMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => deleteMenu.removeItem(object)
            }
        }
    }

    Dialog {
        id: saveDialog
        palette: root.palette

        parent: Overlay.overlay
        anchors.centerIn: parent
        modal: true
        padding: Theme.spacing * 2
        title: qsTr("Save session")
        standardButtons: Dialog.Save | Dialog.Cancel

        onAboutToShow: {
            nameField.text = ""
            nameField.forceActiveFocus()
        }
        onAccepted: {
            if (nameField.text.trim().length > 0)
                Tabs.saveSessionAs(nameField.text.trim())
        }

        ColumnLayout {
            spacing: Theme.spacing

            Label {
                text: qsTr("Saves the open tabs so you can reopen them later.")
            }
            TextField {
                id: nameField
                Layout.fillWidth: true
                placeholderText: qsTr("Session name")
                onAccepted: saveDialog.accept()
            }
        }
    }
}
