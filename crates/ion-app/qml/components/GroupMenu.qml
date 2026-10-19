import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Actions for one tab group, from its header in either strip: name, color,
// fold, add a tab, ungroup or close it.
Item {
    id: root

    // The group the menu or dialog is for, as `{id, name, color, collapsed,
    // tabs}`, fixed when it opens.
    property var group: null

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

    function find(id) {
        return JSON.parse(Tabs.groups(Tabs.workspace)).find(g => g.id === id) ?? null
    }

    function openFor(id, anchor) {
        group = find(id)
        if (group)
            menu.popup(anchor, 0, anchor.height)
    }

    // Ask for a name for group `id`, as right after creating it.
    function rename(id) {
        group = find(id)
        if (group)
            nameDialog.open()
    }

    // Start a group with the tab at `row`, in the next color, and ask for its name.
    function create(row) {
        const colors = Theme.workspaceColors
        const count = JSON.parse(Tabs.groups(Tabs.workspaceAt(row))).length
        const id = Tabs.createGroup(row, "", colors[(count + 5) % colors.length].value)
        if (id >= 0)
            rename(id)
    }

    // The last tab of the group, to open a new one after it.
    function lastRow(id) {
        let last = -1
        for (let i = 0; i < Tabs.count; i++) {
            if (Tabs.groupAt(i) === id)
                last = i
        }
        return last
    }

    IonMenu {
        id: menu
        palette: root.palette

        IonMenuItem {
            text: qsTr("Name group…")
            onTriggered: nameDialog.open()
        }

        IonMenu {
            id: colorMenu
            palette: root.palette
            title: qsTr("Color")

            Instantiator {
                model: Theme.workspaceColors
                delegate: IonMenuItem {
                    required property var modelData
                    text: modelData.name
                    checkable: true
                    checked: root.group?.color === modelData.value
                    onTriggered: Tabs.setGroupColor(root.group.id, modelData.value)
                }
                onObjectAdded: (index, object) => colorMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => colorMenu.removeItem(object)
            }
        }

        IonMenuItem {
            text: root.group?.collapsed ? qsTr("Show tabs") : qsTr("Fold group")
            onTriggered: Tabs.setGroupCollapsed(root.group.id, !root.group.collapsed)
        }
        IonMenuItem {
            text: qsTr("New tab in group")
            onTriggered: {
                const id = root.group.id
                const at = Tabs.openTabAt(root.lastRow(id) + 1, "", true)
                Tabs.setGroup(at, id)
            }
        }

        IonMenuSeparator {}

        IonMenuItem {
            text: qsTr("Ungroup")
            onTriggered: Tabs.ungroup(root.group.id)
        }
        IonMenuItem {
            text: qsTr("Close group")
            onTriggered: Tabs.closeGroup(root.group.id)
        }
    }

    IonDialog {
        id: nameDialog
        palette: root.palette

        title: qsTr("Name group")
        standardButtons: Dialog.Ok | Dialog.Cancel

        onAboutToShow: {
            nameField.text = root.group?.name ?? ""
            nameField.selectAll()
            nameField.forceActiveFocus()
        }
        onAccepted: if (root.group) Tabs.renameGroup(root.group.id, nameField.text)

        ColumnLayout {
            anchors.left: parent.left
            anchors.right: parent.right
            spacing: Theme.spacing * 2

            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                color: Theme.textMuted
                text: qsTr("Optional. Without a name the group shows only its color.")
            }
            IonTextField {
                id: nameField
                Layout.fillWidth: true
                placeholderText: qsTr("For example Trip planning")
                onAccepted: nameDialog.accept()
            }
        }
    }
}
