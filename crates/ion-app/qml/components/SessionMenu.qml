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

    IonMenu {
        id: menu
        palette: root.palette

        onAboutToShow: root.names = Tabs.sessionNames()

        IonMenuItem {
            text: qsTr("Reopen closed tab")
            enabled: Tabs.closedCount > 0
            onTriggered: Tabs.reopenClosedTab()
        }
        IonMenuItem {
            text: qsTr("Vertical tabs")
            checkable: true
            // Writes `ui.tabs` to the local config, like any other setting.
            checked: Config.tabLayout === "vertical"
            onTriggered: Config.set("ui.tabs", checked ? "vertical" : "horizontal")
        }

        IonMenuSeparator {}

        IonMenuItem {
            text: qsTr("Save session as…")
            onTriggered: saveDialog.open()
        }

        IonMenu {
            id: openMenu
            palette: root.palette
            title: qsTr("Open session")
            enabled: root.names.length > 0

            Instantiator {
                model: root.names
                delegate: IonMenuItem {
                    required property string modelData
                    text: modelData
                    onTriggered: Tabs.openSession(modelData)
                }
                onObjectAdded: (index, object) => openMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => openMenu.removeItem(object)
            }
        }

        IonMenu {
            id: deleteMenu
            palette: root.palette
            title: qsTr("Delete session")
            enabled: root.names.length > 0

            Instantiator {
                model: root.names
                delegate: IonMenuItem {
                    required property string modelData
                    text: modelData
                    onTriggered: Tabs.deleteSession(modelData)
                }
                onObjectAdded: (index, object) => deleteMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => deleteMenu.removeItem(object)
            }
        }
    }

    IonDialog {
        id: saveDialog
        palette: root.palette

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
            anchors.left: parent.left
            anchors.right: parent.right
            spacing: Theme.spacing * 2

            Text {
                Layout.fillWidth: true
                text: qsTr("Saves the open tabs so you can reopen them later.")
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize
                wrapMode: Text.WordWrap
            }
            IonTextField {
                id: nameField
                Layout.fillWidth: true
                placeholderText: qsTr("Session name")
                onAccepted: saveDialog.accept()
            }
        }
    }
}
