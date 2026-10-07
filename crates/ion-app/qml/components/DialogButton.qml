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
        color: button.primary ? Theme.background : Theme.text
        font.pixelSize: Theme.fontSize
        font.bold: button.primary
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
    background: Rectangle {
        radius: Theme.radius
        color: button.primary ? Theme.accent : button.hovered ? Theme.surfaceHover : Theme.surfaceRaised
        opacity: !button.enabled ? 0.5 : button.primary && button.down ? 0.8 : 1
    }
}
