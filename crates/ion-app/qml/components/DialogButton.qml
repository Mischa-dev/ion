import QtQuick
import QtQuick.Controls
import Ion

// A button for Ion's prompts and dialogs; `primary` marks the default action.
Button {
    id: button

    property bool primary: false

    implicitHeight: Theme.urlBarHeight
    leftPadding: Theme.spacing * 3
    rightPadding: Theme.spacing * 3
    focusPolicy: Qt.NoFocus

    contentItem: Text {
        text: button.text
        color: button.primary ? Theme.onAccent : Theme.text
        font.pixelSize: Theme.fontSize
        font.weight: Theme.weightMedium
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
    background: Rectangle {
        radius: Theme.radius
        color: button.primary ? Theme.accent : button.hovered ? Theme.surfaceHover : Theme.surfaceRaised
        opacity: !button.enabled ? 0.5 : button.primary && button.down ? 0.85 : 1
        scale: button.down ? 0.97 : 1

        Behavior on color { ColorAnimation { duration: Theme.animationFastMs } }
        Behavior on scale { NumberAnimation { duration: Theme.animationFastMs; easing.type: Theme.easeOut } }

        // A light sheen on the primary button, top to bottom.
        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            visible: button.primary
            gradient: Gradient {
                GradientStop { position: 0; color: button.hovered ? Theme.sheenHover : Theme.sheen }
                GradientStop { position: 1; color: Theme.sheenEnd }
            }
        }
    }
}
