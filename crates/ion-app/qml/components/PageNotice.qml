import QtQuick
import Ion

// A short message over the top of a tab's page ("Screenshot saved", "No
// article found"), fading out by itself.
Rectangle {
    id: root

    function show(message) {
        label.text = message
        opacity = 1
        hideTimer.restart()
    }

    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: parent.top
    anchors.topMargin: Theme.spacing * 3
    // Above the page content, which the view adds as a child of its own.
    z: 1
    width: Math.min(label.implicitWidth + Theme.spacing * 4, parent.width - Theme.spacing * 4)
    height: label.implicitHeight + Theme.spacing * 2
    radius: Theme.radius
    color: Theme.surfaceRaised
    border.width: 1
    border.color: Theme.border
    opacity: 0
    visible: opacity > 0

    Behavior on opacity { NumberAnimation { duration: Theme.animationMs } }

    Text {
        id: label
        anchors.centerIn: parent
        width: parent.width - Theme.spacing * 4
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideMiddle
        color: Theme.text
        font.pixelSize: Theme.fontSize
    }

    Timer {
        id: hideTimer
        interval: Theme.noticeMs
        onTriggered: root.opacity = 0
    }
}
