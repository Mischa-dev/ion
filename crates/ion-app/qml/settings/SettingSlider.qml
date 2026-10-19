import QtQuick
import QtQuick.Controls
import Ion

// A slider for number settings: thin track, accent fill, round handle with
// a focus ring.
Slider {
    id: control

    implicitHeight: Theme.urlBarHeight
    padding: 0
    snapMode: Slider.SnapAlways

    background: Rectangle {
        x: control.leftPadding
        y: control.topPadding + (control.availableHeight - height) / 2
        width: control.availableWidth
        height: Theme.spacing - 2
        radius: height / 2
        color: Theme.surfaceRaised

        Rectangle {
            width: control.visualPosition * parent.width
            height: parent.height
            radius: parent.radius
            color: Theme.accent
        }
    }

    handle: Rectangle {
        x: control.leftPadding + control.visualPosition * (control.availableWidth - width)
        y: control.topPadding + (control.availableHeight - height) / 2
        width: Theme.iconSize
        height: width
        radius: width / 2
        color: control.pressed ? Theme.surfaceHover : Theme.text
        border.width: control.visualFocus ? Theme.hairline * 2 : 0
        border.color: Theme.accent
    }
}
