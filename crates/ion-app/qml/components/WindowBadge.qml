import QtQuick
import QtQuick.Controls
import Ion

// A pill by the toolbar buttons naming a private window ("Private") or a
// non-default profile ("Work"), so it's clear where you are browsing.
// Clicking it lists the profiles.
AbstractButton {
    id: badge

    readonly property string kind: Privacy.windowKind()

    signal profilesRequested()

    visible: kind !== "normal"
    implicitHeight: Theme.urlBarHeight - Theme.spacing
    leftPadding: Theme.spacing * 1.5
    rightPadding: Theme.spacing * 1.5
    focusPolicy: Qt.NoFocus
    onClicked: profilesRequested()

    contentItem: Text {
        text: badge.kind === "private" ? qsTr("◐ Private") : Privacy.profileName()
        textFormat: Text.PlainText
        color: badge.kind === "private" ? Theme.onAccent : Theme.text
        font.pixelSize: Theme.fontSize - 1
        font.weight: Font.DemiBold
        verticalAlignment: Text.AlignVCenter
    }
    background: Rectangle {
        radius: height / 2
        color: badge.kind === "private" ? Theme.accent : badge.hovered ? Theme.surfaceHover : Theme.surfaceRaised
        opacity: badge.down ? 0.85 : 1
    }

    ToolTip.visible: hovered
    ToolTip.delay: 600
    ToolTip.text: kind === "private"
        ? qsTr("Private window: no history, and cookies and site data are forgotten when it closes")
        : qsTr("Profile %1: its own sign-ins, history and passwords").arg(Privacy.profileName())
}
