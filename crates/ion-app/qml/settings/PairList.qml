import QtQuick
import QtQuick.Layouts
import Ion

// Rows of two strings (a name and an address) with a × on each, plus fields
// to add one. Used by BangsEditor and NewTabTiles; they own the saving.
ColumnLayout {
    id: list

    // [{ name, value, removable }]
    property var items: []
    property string namePrefix: ""
    property string namePlaceholder
    property string valuePlaceholder
    property string addText: qsTr("Add")
    // Set by the owner when an add is refused.
    property string error: ""

    // Return true once added, to clear the fields.
    property var add: (name, value) => false
    signal removeClicked(int index)

    spacing: Theme.spacing

    Repeater {
        model: list.items
        delegate: RowLayout {
            id: entry
            required property var modelData
            required property int index
            Layout.fillWidth: true
            spacing: Theme.spacing * 2

            Text {
                Layout.preferredWidth: Theme.tabMaxWidth * 0.6
                text: list.namePrefix + entry.modelData.name
                color: Theme.text
                font.pixelSize: Theme.fontSize
                elide: Text.ElideRight
                textFormat: Text.PlainText
            }
            Text {
                Layout.fillWidth: true
                text: entry.modelData.value
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize - 1
                elide: Text.ElideMiddle
                textFormat: Text.PlainText
            }
            Text {
                visible: entry.modelData.removable !== false
                text: "×"
                color: removeMouse.containsMouse ? Theme.danger : Theme.textMuted
                font.pixelSize: Theme.fontSize + 2

                MouseArea {
                    id: removeMouse
                    anchors.fill: parent
                    anchors.margins: -Theme.spacing
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: list.removeClicked(entry.index)
                }
            }
        }
    }

    RowLayout {
        Layout.fillWidth: true
        Layout.topMargin: list.items.length > 0 ? Theme.spacing : 0
        spacing: Theme.spacing

        IonTextField {
            id: nameField
            Layout.preferredWidth: Theme.tabMaxWidth * 0.6
            placeholderText: list.namePlaceholder
            onAccepted: valueField.forceActiveFocus()
        }
        IonTextField {
            id: valueField
            Layout.fillWidth: true
            placeholderText: list.valuePlaceholder
            onAccepted: addButton.clicked()
        }
        DialogButton {
            id: addButton
            text: list.addText
            enabled: nameField.text.trim().length > 0 && valueField.text.trim().length > 0
            onClicked: {
                if (list.add(nameField.text.trim(), valueField.text.trim())) {
                    nameField.text = ""
                    valueField.text = ""
                    nameField.forceActiveFocus()
                }
            }
        }
    }

    Text {
        Layout.fillWidth: true
        visible: list.error.length > 0
        text: list.error
        color: Theme.danger
        font.pixelSize: Theme.fontSize - 1
        wrapMode: Text.Wrap
    }
}
