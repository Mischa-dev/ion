import QtQuick
import QtQuick.Controls
import Ion

// Flat toolbar button. Shows `iconName` (a name from IconSet.qml) when set, else
// the text in `glyph` (for labels such as the zoom level).
ToolButton {
    id: control

    property string iconName
    property string glyph
    property string tip
    // On, like a toggled panel or a lit feature: tinted with the accent.
    property bool active: false
    property color iconColor: !enabled ? Theme.textMuted : active ? Theme.accent : Theme.text
    property real iconSize: Theme.iconSize

    implicitWidth: Theme.urlBarHeight
    implicitHeight: Theme.urlBarHeight
    focusPolicy: Qt.NoFocus

    contentItem: Item {
        opacity: control.enabled ? 1 : 0.4
        scale: control.down ? 0.92 : 1

        Behavior on scale { NumberAnimation { duration: Theme.animationFastMs; easing.type: Theme.easeOut } }

        Icon {
            anchors.centerIn: parent
            visible: control.iconName.length > 0
            name: control.iconName
            size: control.iconSize
            color: control.iconColor
        }
        Text {
            anchors.fill: parent
            visible: control.iconName.length === 0
            text: control.glyph
            color: control.iconColor
            font.pixelSize: control.glyph.length > 2 ? Theme.fontSizeSmall : Theme.fontSize + 3
            font.weight: control.glyph.length > 2 ? Theme.weightMedium : Font.Normal
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    background: Rectangle {
        radius: Theme.radius
        color: control.down ? Theme.pressOverlay
             : control.active ? Theme.accentSoft
             : control.hovered ? Theme.hoverOverlay : "transparent"

        Behavior on color { ColorAnimation { duration: Theme.animationFastMs } }
    }

    // A click hides the tip until the pointer leaves, so it doesn't sit on
    // top of the menu or popup the button just opened.
    property bool tipDismissed: false
    onClicked: tipDismissed = true
    onHoveredChanged: if (!hovered) tipDismissed = false

    ToolTip.visible: hovered && !tipDismissed && tip.length > 0
    ToolTip.text: tip
    ToolTip.delay: 600
}
