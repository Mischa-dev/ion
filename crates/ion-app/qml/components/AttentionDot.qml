import QtQuick
import Ion

// Marks a tab that wants the person's attention, such as one whose page
// opened a dialog while it was in the background. Pulses gently.
Rectangle {
    width: Theme.spacing * 1.5
    height: width
    radius: width / 2
    color: Theme.accent
    border.color: Theme.background
    border.width: 1

    SequentialAnimation on opacity {
        running: visible
        loops: Animation.Infinite
        alwaysRunToEnd: true
        NumberAnimation { to: Theme.dimmedOpacity; duration: Theme.animationMs * 4 }
        NumberAnimation { to: 1; duration: Theme.animationMs * 4 }
    }
}
