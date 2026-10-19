import QtQuick
import QtQuick.Controls
import Ion

// A pinned tab or Essential as a square tile showing only its icon, in
// either tab strip. Same mouse handling as a full tab.
Rectangle {
    id: tab

    required property int row
    required property string title
    required property string url
    required property string icon
    required property bool suspended
    required property bool essential
    // The strip's list, for drag reordering, and its current row.
    required property var list
    required property int currentIndex
    property var view: null

    readonly property bool current: row === currentIndex

    signal activated(int index)
    signal closeRequested(int index)
    signal moveRequested(int from, int to)
    signal menuRequested(int index)
    signal draggedOut(int index, point scenePos)
    signal droppedOut(int index, point scenePos)

    radius: Theme.radius
    z: tabMouse.dragging ? 1 : 0
    transform: Translate { x: tabMouse.dragX; y: tabMouse.dragY }
    color: tabMouse.dragging ? Theme.surfaceHover
         : current ? Theme.surface : tabMouse.containsMouse ? Theme.surfaceRaised : Theme.background
    border.width: essential ? Theme.hairline : 0
    border.color: current ? Theme.accent : Theme.border

    Behavior on color { ColorAnimation { duration: Theme.animationMs } }

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

    TabIcon {
        id: favicon
        anchors.centerIn: parent
        view: tab.view
        savedIcon: tab.icon
        opacity: tab.suspended ? Theme.dimmedOpacity : 1
    }
    // Sites without an icon show their first letter.
    Text {
        anchors.centerIn: parent
        visible: favicon.implicitWidth === 0
        text: {
            const name = tab.title.length > 0 ? tab.title : tab.url.replace(/^[a-z]+:\/\/(www\.)?/, "")
            return name.length > 0 ? name[0].toUpperCase() : "•"
        }
        color: tab.current ? Theme.text : Theme.textMuted
        font.pixelSize: Theme.fontSize
        font.bold: true
    }

    // Sound from a pinned tab shows as a dot; the menu mutes it.
    Rectangle {
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: Theme.spacing / 2
        width: Theme.spacing
        height: width
        radius: width / 2
        color: tab.view?.audioMuted ? Theme.textMuted : Theme.accent
        visible: (tab.view?.recentlyAudible ?? false) || (tab.view?.audioMuted ?? false)
    }

    AttentionDot {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.margins: Theme.spacing / 2
        visible: tab.view?.needsAttention ?? false
    }

    ToolTip.visible: tabMouse.containsMouse && !tabMouse.dragging
    ToolTip.text: tab.title.length > 0 ? tab.title : tab.url
    ToolTip.delay: Theme.hoverDelayMs * 2
}
