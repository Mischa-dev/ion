import QtQuick
import Ion

// A tab's speaker: shown while its page plays sound or is muted, and
// clicking it mutes or unmutes the tab.
IconButton {
    id: control

    property var view   // BrowserTab, may be null

    readonly property bool muted: view?.audioMuted ?? false
    // Playing sound, or muted: the button is shown.
    readonly property bool audible: muted || (view?.recentlyAudible ?? false)

    visible: audible
    iconName: muted ? "mute" : "volume"
    iconSize: Theme.iconSize - 2
    iconColor: Theme.textMuted
    tip: muted ? qsTr("Unmute tab") : qsTr("Mute tab")
    implicitWidth: Theme.tabHeight - Theme.spacing * 2
    implicitHeight: Theme.tabHeight - Theme.spacing * 2
    onClicked: if (view) view.audioMuted = !view.audioMuted
}
