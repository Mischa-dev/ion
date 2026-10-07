import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Shown while every agent is stopped (the stop-agents shortcut or palette
// command), so it is never a mystery why agents won't act, and resuming is
// one click away. Sits at the bottom of the window, above the page.
Rectangle {
    id: notice

    visible: Safety.stopped
    z: 20
    anchors.bottom: parent.bottom
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.bottomMargin: Theme.spacing * 3
    width: row.implicitWidth + Theme.spacing * 4
    height: Theme.urlBarHeight + Theme.spacing
    radius: height / 2
    color: Theme.surfaceRaised
    border.color: Theme.warning
    border.width: 1

    RowLayout {
        id: row
        anchors.centerIn: parent
        spacing: Theme.spacing * 2

        Rectangle {
            implicitWidth: Theme.spacing + 2
            implicitHeight: implicitWidth
            radius: implicitWidth / 2
            color: Theme.warning
        }
        Text {
            text: qsTr("Agents stopped")
            color: Theme.text
            font.pixelSize: Theme.fontSize
        }
        Button {
            id: resume
            text: qsTr("Resume")
            focusPolicy: Qt.NoFocus
            implicitHeight: Theme.urlBarHeight - Theme.spacing
            leftPadding: Theme.spacing * 2
            rightPadding: Theme.spacing * 2
            onClicked: Safety.resumeAgents()
            contentItem: Text {
                text: resume.text
                color: Theme.text
                font.pixelSize: Theme.fontSize
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            background: Rectangle {
                radius: height / 2
                color: resume.hovered ? Theme.surfaceHover : Theme.surface
            }
        }
    }
}
