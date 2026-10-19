import QtQuick
import QtQuick.Controls
import Ion

// Shown on a tab while its page plays sound, or while it is muted. A click
// mutes or unmutes the tab without switching to it.
ToolButton {
    id: control

    property var view   // the tab's BrowserTab, may be null

    readonly property bool muted: view?.audioMuted ?? false
    readonly property bool audible: view?.recentlyAudible ?? false

    visible: muted || audible
    implicitWidth: Theme.iconSize + Theme.spacing
    implicitHeight: Theme.iconSize + Theme.spacing
    focusPolicy: Qt.NoFocus
    padding: 0

    contentItem: SpeakerIcon {
        muted: control.muted
        color: control.muted ? Theme.textMuted : control.hovered ? Theme.text : Theme.accent
    }

    background: Rectangle {
        radius: Theme.radius
        color: control.down ? Theme.surfaceRaised : control.hovered ? Theme.surfaceHover : "transparent"
    }

    onClicked: if (view) view.audioMuted = !view.audioMuted

    ToolTip.visible: hovered
    ToolTip.text: muted ? qsTr("Unmute tab") : qsTr("Mute tab")
    ToolTip.delay: 600
}
