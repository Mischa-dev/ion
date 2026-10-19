import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// One tab in either strip: icon, title, speaker and close button, with the
// mouse handling of TabDragArea. The strips lay these out and add group
// headers around them.
Rectangle {
    id: tab

    required property int row
    required property string title
    required property string url
    required property string icon
    required property bool suspended
    // The strip's list, for drag reordering, and its current row.
    required property var list
    required property int currentIndex
    property var view: null
    // Titles are hidden in the collapsed sidebar.
    property bool showTitle: true
    // Untitled tabs show their address instead of "New Tab" (the sidebar).
    property bool urlAsTitle: false
    // Hover previews open to the right in the sidebar, below in the top strip.
    property bool previewBeside: false
    // In a split view, and whether that view is on screen.
    property bool inSplit: false
    property bool splitShowing: false

    readonly property bool current: row === currentIndex
    readonly property bool hovered: tabMouse.containsMouse || closeButton.hovered || audio.hovered

    signal activated(int index)
    signal closeRequested(int index)
    signal moveRequested(int from, int to)
    signal menuRequested(int index)
    signal draggedOut(int index, point scenePos)
    signal droppedOut(int index, point scenePos)

    radius: Theme.radius
    z: tabMouse.dragging ? 2 : current ? 1 : 0
    transform: Translate { x: tabMouse.dragX; y: tabMouse.dragY }
    // The current tab is a raised card; others are flat until hovered.
    color: current || tabMouse.dragging ? Theme.surface
         : tabMouse.containsMouse || splitShowing ? Theme.hoverOverlay : "transparent"
    border.width: current ? Theme.hairline : 0
    border.color: Theme.border

    Behavior on color { ColorAnimation { duration: Theme.animationMs } }

    IonShadow {
        anchors.fill: parent
        z: -1
        visible: tab.current || tabMouse.dragging
        radius: tab.radius
        spread: tabMouse.dragging ? Theme.shadowRadius / 2 : Theme.spacing / 2
        offset: Theme.hairline
        strength: tabMouse.dragging ? 0.8 : 0.35
    }

    TabPreview {
        view: tab.view
        title: tab.title
        url: tab.url
        suspended: tab.suspended
        current: tab.current
        beside: tab.previewBeside
        wanted: tabMouse.containsMouse && !tabMouse.pressed && !closeButton.hovered && !audio.hovered
    }

    TabDragArea {
        id: tabMouse
        list: tab.list
        index: tab.row
        onActivated: index => tab.activated(index)
        onCloseRequested: index => tab.closeRequested(index)
        onMoveRequested: (from, to) => tab.moveRequested(from, to)
        onMenuRequested: index => tab.menuRequested(index)
        onDraggedOut: (index, pos) => tab.draggedOut(index, pos)
        onDroppedOut: (index, pos) => tab.droppedOut(index, pos)
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spacing * 2
        anchors.rightMargin: Theme.spacing
        spacing: Theme.spacing

        TabIcon {
            Layout.alignment: Qt.AlignVCenter
            view: tab.view
            savedIcon: tab.icon
            opacity: tab.suspended ? Theme.dimmedOpacity : 1

            AttentionDot {
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: -Theme.spacing / 2
                visible: tab.view?.needsAttention ?? false
            }
        }

        // Split views are marked so it's clear which tabs open together.
        Icon {
            Layout.alignment: Qt.AlignVCenter
            visible: tab.inSplit && tab.showTitle
            name: "split"
            size: Theme.iconSize - 4
            color: tab.splitShowing ? Theme.accent : Theme.textMuted
        }

        Text {
            Layout.fillWidth: true
            visible: tab.showTitle
            text: tab.title.length > 0 ? tab.title
                : tab.urlAsTitle && tab.url.length > 0 ? tab.url : qsTr("New Tab")
            color: tab.current ? Theme.text : Theme.textMuted
            font.pixelSize: Theme.fontSize
            font.weight: tab.current ? Theme.weightMedium : Font.Normal
            elide: Text.ElideRight
            textFormat: Text.PlainText
        }

        TabAudioButton {
            id: audio
            Layout.alignment: Qt.AlignVCenter
            view: tab.view
            visible: audible && tab.showTitle
        }

        IconButton {
            id: closeButton
            Layout.preferredWidth: Theme.tabHeight - Theme.spacing * 2
            Layout.preferredHeight: Theme.tabHeight - Theme.spacing * 2
            iconName: "close"
            iconSize: Theme.iconSize - 4
            tip: qsTr("Close tab")
            visible: tab.showTitle
            opacity: tab.current || tabMouse.containsMouse || hovered ? 1 : 0
            enabled: opacity > 0
            onClicked: tab.closeRequested(tab.row)

            Behavior on opacity { NumberAnimation { duration: Theme.animationFastMs } }
        }
    }
}
