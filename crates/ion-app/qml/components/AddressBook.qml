import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Saved addresses, the password manager's second page: a list on the left,
// the selected (or a new) address to edit on the right. Ion offers to save
// an address when a form with one is sent, and lists them under address
// fields to fill in.
Item {
    id: root

    property bool active: false
    property string query: ""
    // The address being edited: its id, or 0 for a new one, or -1 for none.
    property int selected: -1
    property string message: ""
    property bool confirmDelete: false

    readonly property var addresses: {
        Passwords.revision
        return active ? JSON.parse(Passwords.addresses(query)) : []
    }
    readonly property var current: addresses.find(a => a.id === selected) ?? null

    readonly property var fields: [
        { key: "name", label: qsTr("Name"), wide: true },
        { key: "organization", label: qsTr("Organization"), wide: true },
        { key: "line1", label: qsTr("Street address"), wide: true },
        { key: "line2", label: qsTr("Apartment, suite, floor"), wide: true },
        { key: "city", label: qsTr("City") },
        { key: "region", label: qsTr("State, province or county") },
        { key: "postalCode", label: qsTr("Postal code") },
        { key: "country", label: qsTr("Country") },
        { key: "email", label: qsTr("Email") },
        { key: "phone", label: qsTr("Phone") }
    ]

    function reset() {
        message = ""
        select(-1)
    }

    // Fills the editor from address `id` (0: an empty new one). Typed text
    // breaks bindings, so the fields are set by hand.
    function select(id) {
        selected = id
        confirmDelete = false
        const a = current ?? {}
        const lines = (a.street ?? "").split("\n")
        const values = Object.assign({}, a, { line1: lines[0] ?? "", line2: lines.slice(1).join(", ") })
        for (let i = 0; i < editors.count; i++) {
            const editor = editors.itemAt(i)
            editor.text = values[editor.key] ?? ""
        }
    }

    function save() {
        const address = { id: Math.max(0, selected), used: current?.used ?? 0 }
        for (let i = 0; i < editors.count; i++) {
            const editor = editors.itemAt(i)
            address[editor.key] = editor.text.trim()
        }
        address.street = [address.line1, address.line2].filter(l => l.length > 0).join("\n")
        delete address.line1
        delete address.line2
        const id = Passwords.saveAddress(JSON.stringify(address))
        if (id >= 0) {
            message = qsTr("Saved")
            select(id)
        }
    }

    component Caption: Label {
        color: Theme.textMuted
        font.pixelSize: Theme.fontSize - 1
        wrapMode: Text.Wrap
        Layout.fillWidth: true
    }

    RowLayout {
        anchors.fill: parent
        spacing: Theme.spacing * 2

        ColumnLayout {
            Layout.preferredWidth: Theme.tabMaxWidth + Theme.spacing * 10
            Layout.maximumWidth: Layout.preferredWidth
            Layout.fillHeight: true
            spacing: Theme.spacing

            ListView {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: root.addresses
                ScrollBar.vertical: ScrollBar {}

                delegate: ItemDelegate {
                    id: row
                    required property var modelData
                    width: ListView.view.width
                    implicitHeight: Theme.tabHeight + Theme.spacing * 2.5
                    leftPadding: Theme.spacing * 1.5
                    rightPadding: Theme.spacing * 1.5
                    focusPolicy: Qt.NoFocus
                    onClicked: root.select(modelData.id)

                    contentItem: ColumnLayout {
                        spacing: 0
                        Label {
                            Layout.fillWidth: true
                            text: row.modelData.name || row.modelData.organization || row.modelData.summary
                            textFormat: Text.PlainText
                            color: Theme.text
                            font.pixelSize: Theme.fontSize
                            elide: Text.ElideRight
                        }
                        Label {
                            Layout.fillWidth: true
                            text: [row.modelData.street.split("\n")[0], row.modelData.city, row.modelData.email]
                                .filter(p => p.length > 0).slice(0, 2).join(", ")
                            textFormat: Text.PlainText
                            color: Theme.textMuted
                            font.pixelSize: Theme.fontSize - 1
                            elide: Text.ElideRight
                        }
                    }
                    background: Rectangle {
                        radius: Theme.radius
                        color: row.modelData.id === root.selected ? Theme.surfaceRaised
                             : row.hovered ? Theme.surfaceHover : "transparent"
                    }
                }

                Label {
                    anchors.centerIn: parent
                    width: parent.width - Theme.spacing * 4
                    visible: list.count === 0
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSize
                    text: root.query.length > 0 ? qsTr("Nothing matches.")
                        : qsTr("No saved addresses yet. Ion offers to save one when you send a form with it, or add one here.")
                }
            }

            DialogButton {
                Layout.fillWidth: true
                text: qsTr("Add an address")
                onClicked: root.select(0)
            }
        }

        Rectangle { Layout.fillHeight: true; implicitWidth: Theme.hairline; color: Theme.border }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.spacing

            Caption {
                visible: root.selected < 0
                Layout.fillHeight: true
                verticalAlignment: Text.AlignVCenter
                horizontalAlignment: Text.AlignHCenter
                text: qsTr("Pick an address to see or change it.")
            }

            GridLayout {
                visible: root.selected >= 0
                Layout.fillWidth: true
                columns: 2
                columnSpacing: Theme.spacing * 1.5
                rowSpacing: Theme.spacing

                Repeater {
                    id: editors
                    model: root.fields

                    delegate: ColumnLayout {
                        required property var modelData
                        readonly property string key: modelData.key
                        property alias text: input.text
                        Layout.columnSpan: modelData.wide ? 2 : 1
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        spacing: Theme.spacing / 2

                        Caption { text: modelData.label }
                        IonTextField {
                            id: input
                            Layout.fillWidth: true
                            onAccepted: root.save()
                        }
                    }
                }
            }

            RowLayout {
                visible: root.selected >= 0
                Layout.fillWidth: true
                Layout.topMargin: Theme.spacing
                spacing: Theme.spacing

                DialogButton {
                    visible: root.current !== null
                    text: root.confirmDelete ? qsTr("Really delete?") : qsTr("Delete")
                    onClicked: {
                        if (!root.confirmDelete) {
                            root.confirmDelete = true
                            return
                        }
                        Passwords.removeAddress(root.selected)
                        root.select(-1)
                    }
                }
                Item { Layout.fillWidth: true }
                DialogButton {
                    text: qsTr("Cancel")
                    onClicked: root.select(-1)
                }
                DialogButton {
                    primary: true
                    text: qsTr("Save")
                    onClicked: root.save()
                }
            }

            Item { Layout.fillHeight: true; visible: root.selected >= 0 }
            Caption {
                visible: root.message.length > 0
                text: root.message
                color: Theme.text
            }
        }
    }
}
