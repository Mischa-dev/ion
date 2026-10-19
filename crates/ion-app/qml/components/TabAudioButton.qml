import QtQuick
import Ion

// A tab's speaker: shown while its page plays sound or is muted, and
// clicking it mutes or unmutes the tab.
IconButton {
    id: control

    property var view   // BrowserTab, may be null

    readonly property bool muted: view?.audioMuted ?? false
    readonly property bool active: muted || (view?.recentlyAudible ?? false)

    visible: active
    glyph: muted ? "🔇" : "🔊"
    tip: muted ? qsTr("Unmute tab") : qsTr("Mute tab")
    implicitWidth: Theme.tabHeight - Theme.spacing * 2
    implicitHeight: Theme.tabHeight - Theme.spacing * 2
    contentItem: Text {
        text: control.glyph
        color: Theme.textMuted
        font.pixelSize: Theme.fontSize - 1
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
    onClicked: if (view) view.audioMuted = !view.audioMuted
}
