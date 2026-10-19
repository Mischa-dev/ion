import QtQuick
import Ion

// One key sequence drawn as a keycap, as in the shortcut editor. `active`
// marks the one being recorded; `removable` shows a × to remove it.
Rectangle {
    id: chip

    property string label
    property bool active: false
    property bool removable: false

    signal clicked()
    signal removeClicked()

    implicitWidth: row.implicitWidth + Theme.spacing * 2
    implicitHeight: Theme.tabHeight - Theme.spacing
    radius: Theme.radius
    color: chipHover.hovered ? Theme.surfaceHover : Theme.surface
    border.width: chip.active ? Theme.hairline * 2 : Theme.hairline
    border.color: chip.active ? Theme.accent : Theme.border

    HoverHandler { id: chipHover; cursorShape: Qt.PointingHandCursor }
    // The × has its own handler; a tap on it only removes.
    TapHandler { onTapped: if (!removeHover.hovered) chip.clicked() }

    Row {
        id: row
        anchors.centerIn: parent
        spacing: Theme.spacing

        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: chip.label
            color: chip.active ? Theme.accent : Theme.text
            font.pixelSize: Theme.fontSize - 1
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: chip.removable && !chip.active
            text: "×"
            color: removeHover.hovered ? Theme.danger : Theme.textMuted
            font.pixelSize: Theme.fontSize

            HoverHandler { id: removeHover }
            TapHandler { onTapped: chip.removeClicked() }
        }
    }
}
