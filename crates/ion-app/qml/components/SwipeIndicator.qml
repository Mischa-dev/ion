import QtQuick
import Ion

// The arrow that slides in from the page's edge during a two-finger swipe
// (`Gestures`). It fills with the accent once letting go would navigate.
Item {
    id: root

    // "back", "forward" or "" (no swipe), and how far along, 0 to 1.
    property string direction
    property real progress

    readonly property bool ready: progress >= 1
    readonly property int size: Theme.urlBarHeight + Theme.spacing * 2

    // Keep showing the last side while fading out after the swipe ends.
    property bool lastBack: true
    onDirectionChanged: if (direction.length > 0) lastBack = direction === "back"

    anchors.fill: parent

    Rectangle {
        id: bubble

        width: root.size
        height: root.size
        radius: width / 2
        y: Math.round((root.height - height) / 2)
        // Slides from just off the edge to a little inside it.
        readonly property real travel: Math.min(1, root.progress) * (width + Theme.spacing * 4)
        x: root.lastBack ? -width + travel : root.width - travel
        opacity: root.direction.length > 0 ? Math.min(1, root.progress * 2) : 0
        scale: root.ready ? 1.1 : 1
        color: root.ready ? Theme.accent : Theme.surfaceRaised
        border.width: Theme.hairline
        border.color: Theme.border

        Behavior on opacity { NumberAnimation { duration: Theme.animationMs } }
        Behavior on scale { NumberAnimation { duration: Theme.animationMs; easing.type: Easing.OutCubic } }
        Behavior on color { ColorAnimation { duration: Theme.animationMs } }

        Text {
            anchors.centerIn: parent
            text: root.lastBack ? "←" : "→"
            color: root.ready ? Theme.onAccent : Theme.text
            font.pixelSize: Theme.fontSize + 6
        }
    }
}
