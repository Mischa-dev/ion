import QtQuick
import QtQuick.Controls
import Ion

// Flat toolbar button using a text glyph until the icon set lands.
ToolButton {
    id: control

    property string glyph
    property string tip

    implicitWidth: Theme.urlBarHeight
    implicitHeight: Theme.urlBarHeight
    focusPolicy: Qt.NoFocus

    contentItem: Text {
        text: control.glyph
        color: control.enabled ? Theme.text : Theme.textMuted
        opacity: control.enabled ? 1 : 0.4
        font.pixelSize: Theme.fontSize + 3
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }

    background: Rectangle {
        radius: Theme.radius
        color: control.down ? Theme.surfaceRaised : control.hovered ? Theme.surfaceHover : "transparent"
    }

    ToolTip.visible: hovered && tip.length > 0
    ToolTip.text: tip
    ToolTip.delay: 600
}
