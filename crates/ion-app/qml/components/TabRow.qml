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

    readonly property bool current: row === currentIndex
    readonly property bool hovered: tabMouse.containsMouse || closeButton.hovered || audio.hovered

    signal activated(int index)
    signal closeRequested(int index)
    signal moveRequested(int from, int to)
    signal dropped(int index)
    signal menuRequested(int index)

    radius: Theme.radius
    z: tabMouse.dragging ? 1 : 0
    color: tabMouse.dragging ? Theme.surfaceHover
         : current ? Theme.surface : tabMouse.containsMouse ? Theme.surfaceRaised : "transparent"

    Behavior on color { ColorAnimation { duration: Theme.animationMs } }

    TabDragArea {
        id: tabMouse
        list: tab.list
        index: tab.row
        onActivated: index => tab.activated(index)
        onCloseRequested: index => tab.closeRequested(index)
        onMoveRequested: (from, to) => tab.moveRequested(from, to)
        onDropped: index => tab.dropped(index)
        onMenuRequested: index => tab.menuRequested(index)
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
        }

        Text {
            Layout.fillWidth: true
            visible: tab.showTitle
            text: tab.title.length > 0 ? tab.title
                : tab.urlAsTitle && tab.url.length > 0 ? tab.url : qsTr("New Tab")
            color: tab.current ? Theme.text : Theme.textMuted
            font.pixelSize: Theme.fontSize
            elide: Text.ElideRight
            textFormat: Text.PlainText
        }

        TabAudioButton {
            id: audio
            Layout.alignment: Qt.AlignVCenter
            view: tab.view
            visible: active && tab.showTitle
        }

        IconButton {
            id: closeButton
            Layout.preferredWidth: Theme.tabHeight - Theme.spacing * 2
            Layout.preferredHeight: Theme.tabHeight - Theme.spacing * 2
            glyph: "×"
            tip: qsTr("Close tab")
            visible: tab.showTitle && (tab.current || tabMouse.containsMouse || hovered)
            onClicked: tab.closeRequested(tab.row)
        }
    }
}
