import QtQuick
import QtQuick.Controls
import Ion

// A single-line text input in Ion's look, like the URL bar's: a quiet filled
// field that lifts to the page color with an accent ring while focused.
TextField {
    id: field

    implicitHeight: Theme.urlBarHeight
    color: Theme.text
    placeholderTextColor: Theme.textMuted
    selectionColor: Theme.accent
    selectedTextColor: Theme.onAccent
    font.pixelSize: Theme.fontSize
    leftPadding: Theme.spacing * 2
    rightPadding: Theme.spacing * 2
    verticalAlignment: TextInput.AlignVCenter
    selectByMouse: true

    background: Rectangle {
        radius: Theme.radius
        color: field.activeFocus ? Theme.surface : field.hovered ? Theme.surfaceHover : Theme.surfaceRaised
        border.width: field.activeFocus ? Theme.hairline * 2 : 0
        border.color: Theme.focusRing

        Behavior on color { ColorAnimation { duration: Theme.animationMs } }
    }
}
