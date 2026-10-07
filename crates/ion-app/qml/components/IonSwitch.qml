import QtQuick
import QtQuick.Controls
import Ion

// An on/off switch in Ion's look: accent track when on, a ring on keyboard
// focus.
Switch {
    id: control

    padding: 0
    spacing: 0
    // Just the indicator; the style's own size would leave it hanging out of
    // the layout slot.
    implicitWidth: indicator.implicitWidth + leftPadding + rightPadding
    implicitHeight: indicator.implicitHeight + topPadding + bottomPadding

    indicator: Rectangle {
        implicitWidth: Theme.iconSize * 2
        implicitHeight: Theme.iconSize + Theme.hairline * 2
        x: control.leftPadding
        y: control.topPadding + (control.availableHeight - height) / 2
        radius: height / 2
        color: control.checked ? Theme.accent
             : control.hovered ? Theme.surfaceHover : Theme.background
        border.width: control.visualFocus ? Theme.hairline * 2
                    : control.checked ? 0 : Theme.hairline
        border.color: control.visualFocus ? (control.checked ? Theme.text : Theme.accent)
                    : Theme.border
        opacity: control.enabled ? 1 : Theme.dimmedOpacity

        Behavior on color { ColorAnimation { duration: Theme.animationMs } }

        Rectangle {
            width: parent.height - Theme.hairline * 6
            height: width
            radius: width / 2
            anchors.verticalCenter: parent.verticalCenter
            x: control.checked ? parent.width - width - Theme.hairline * 3 : Theme.hairline * 3
            color: control.checked ? Theme.onAccent : Theme.textMuted

            Behavior on x { NumberAnimation { duration: Theme.animationMs; easing.type: Easing.OutCubic } }
        }
    }

    contentItem: Item {}
}
