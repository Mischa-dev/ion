import QtQuick
import Ion

// Toolbar button for the downloads panel. Appears once something has been
// downloaded and shows the progress of running downloads along its bottom.
IconButton {
    id: button

    property DownloadsPanel panel
    readonly property int activeCount: panel?.activeCount ?? 0
    readonly property real progress: panel?.activeProgress ?? -1

    visible: (panel?.items.length ?? 0) > 0
    glyph: "↓"
    tip: activeCount > 0 ? qsTr("Downloads (%1 running)").arg(activeCount) : qsTr("Downloads")
    onClicked: panel.toggle()

    Rectangle {
        visible: button.activeCount > 0
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        anchors.margins: Theme.spacing / 2
        height: 2
        radius: 1
        color: Theme.accent
        width: (parent.width - Theme.spacing) * (button.progress < 0 ? 1 : button.progress)
        opacity: button.progress < 0 ? 0.4 : 1
    }
}
