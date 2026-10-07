import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Workspace actions behind the workspace button of the tab strips: switch,
// create, rename, recolor, set an icon for and delete workspaces.
Item {
    id: root

    // Workspaces as `{id, name, color, icon, tabs}`, refreshed every time the menu
    // opens, and the current one.
    property var workspaces: []
    readonly property var current: workspaces.find(w => w.id === Tabs.workspace) ?? null

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

    // Ask for a name, then create a workspace with it and switch to it. With
    // `moveRow`, the tab at that row moves there first.
    function create(moveRow) {
        nameDialog.renaming = false
        nameDialog.moveRow = moveRow ?? -1
        nameDialog.open()
    }

    Menu {
        id: menu
        palette: root.palette

        onAboutToShow: root.workspaces = JSON.parse(Tabs.workspaces())

        Instantiator {
            model: root.workspaces
            delegate: MenuItem {
                required property var modelData
                text: modelData.icon.length > 0 ? modelData.icon + "  " + modelData.name : modelData.name
                checkable: true
                checked: modelData.id === Tabs.workspace
                onTriggered: Tabs.switchWorkspace(modelData.id)
            }
            onObjectAdded: (index, object) => menu.insertItem(index, object)
            onObjectRemoved: (index, object) => menu.removeItem(object)
        }

        MenuSeparator {}

        MenuItem {
            text: qsTr("New workspace…")
            onTriggered: root.create()
        }
        MenuItem {
            text: qsTr("Rename workspace…")
            onTriggered: {
                nameDialog.renaming = true
                nameDialog.open()
            }
        }

        MenuItem {
            text: qsTr("Icon…")
            onTriggered: iconDialog.open()
        }

        Menu {
            id: colorMenu
            palette: root.palette
            title: qsTr("Color")

            Instantiator {
                model: Theme.workspaceColors
                delegate: MenuItem {
                    required property var modelData
                    text: modelData.name
                    checkable: true
                    checked: root.current?.color === modelData.value
                    onTriggered: Tabs.setWorkspaceColor(Tabs.workspace, modelData.value)
                }
                onObjectAdded: (index, object) => colorMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => colorMenu.removeItem(object)
            }
        }

        MenuItem {
            text: qsTr("Delete workspace…")
            enabled: root.workspaces.length > 1
            onTriggered: deleteDialog.open()
        }
    }

    Dialog {
        id: nameDialog
        palette: root.palette

        // Renaming the current workspace rather than creating one.
        property bool renaming: false
        // The tab to move into the new workspace, or -1.
        property int moveRow: -1

        parent: Overlay.overlay
        anchors.centerIn: parent
        modal: true
        padding: Theme.spacing * 2
        title: renaming ? qsTr("Rename workspace") : qsTr("New workspace")
        standardButtons: Dialog.Ok | Dialog.Cancel

        onAboutToShow: {
            nameField.text = renaming ? (root.current?.name ?? "") : ""
            nameField.selectAll()
            nameField.forceActiveFocus()
        }
        onAccepted: {
            const name = nameField.text.trim()
            if (renaming) {
                Tabs.renameWorkspace(Tabs.workspace, name)
            } else {
                // New workspaces take the next color in the list.
                const colors = Theme.workspaceColors
                const count = JSON.parse(Tabs.workspaces()).length
                const id = Tabs.addWorkspace(name, colors[count % colors.length].value)
                if (moveRow >= 0)
                    Tabs.moveToWorkspace(moveRow, id)
                Tabs.switchWorkspace(id)
            }
        }

        ColumnLayout {
            spacing: Theme.spacing

            Label {
                text: nameDialog.renaming ? qsTr("A new name for this workspace.")
                                          : qsTr("A workspace keeps its own set of tabs.")
            }
            TextField {
                id: nameField
                Layout.fillWidth: true
                placeholderText: qsTr("Workspace name")
                onAccepted: nameDialog.accept()
            }
        }
    }

    Dialog {
        id: iconDialog
        palette: root.palette

        parent: Overlay.overlay
        anchors.centerIn: parent
        modal: true
        padding: Theme.spacing * 2
        title: qsTr("Workspace icon")
        standardButtons: Dialog.Ok | Dialog.Cancel

        onAboutToShow: {
            iconField.text = root.current?.icon ?? ""
            iconField.selectAll()
            iconField.forceActiveFocus()
        }
        onAccepted: Tabs.setWorkspaceIcon(Tabs.workspace, iconField.text)

        ColumnLayout {
            spacing: Theme.spacing

            Label {
                text: qsTr("An emoji or a few letters, shown instead of the color dot. Leave empty for the dot.")
            }
            TextField {
                id: iconField
                Layout.fillWidth: true
                // No maximumLength: it counts UTF-16 units and would cut emoji
                // apart. Tabs.setWorkspaceIcon caps whole characters instead.
                placeholderText: qsTr("For example 💼")
                onAccepted: iconDialog.accept()
            }
        }
    }

    Dialog {
        id: deleteDialog
        palette: root.palette

        parent: Overlay.overlay
        anchors.centerIn: parent
        modal: true
        padding: Theme.spacing * 2
        title: qsTr("Delete workspace")
        standardButtons: Dialog.Ok | Dialog.Cancel

        onAccepted: Tabs.deleteWorkspace(Tabs.workspace)

        Label {
            text: {
                const tabs = root.current?.tabs ?? 0
                return qsTr("Delete “%1” and close its %n tab(s)? Closed tabs can be reopened one by one.", "", tabs)
                    .arg(root.current?.name ?? "")
            }
        }
    }
}
