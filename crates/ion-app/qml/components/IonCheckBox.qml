import QtQuick
import QtQuick.Controls
import Ion

// A small check box in Ion's look, for options under a prompt's question.
CheckBox {
    id: box

    focusPolicy: Qt.NoFocus
    padding: 0

    indicator: Rectangle {
        implicitWidth: Theme.fontSize + 4
        implicitHeight: implicitWidth
        y: (box.height - height) / 2
        radius: Theme.radius / 2
        color: box.checked ? Theme.accent : Theme.surfaceRaised
        border.color: Theme.border
        border.width: 1

        Text {
            anchors.centerIn: parent
            visible: box.checked
            text: "✓"
            color: Theme.background
            font.pixelSize: Theme.fontSize - 1
            font.bold: true
        }
    }
    contentItem: Text {
        leftPadding: box.indicator.width + Theme.spacing
        text: box.text
        color: Theme.textMuted
        font.pixelSize: Theme.fontSize - 1
        verticalAlignment: Text.AlignVCenter
        wrapMode: Text.Wrap
    }
}
