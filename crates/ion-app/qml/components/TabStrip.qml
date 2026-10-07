import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Horizontal tab strip over the `Tabs` model. Reports clicks; Main.qml decides
// what they do. VerticalTabStrip.qml is the sidebar variant.
Rectangle {
    id: strip

    required property var tabs   // the Tabs model
    property int currentIndex: 0

    signal activated(int index)
    signal closeRequested(int index)
    signal moveRequested(int from, int to)
    signal newTabRequested()
    signal menuRequested(Item anchor)

    implicitHeight: Theme.tabHeight + Theme.spacing
    color: Theme.background

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        anchors.topMargin: Theme.spacing
        spacing: Theme.spacing / 2

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: ListView.Horizontal
            spacing: Theme.spacing / 2
            clip: true
            interactive: contentWidth > width
            model: strip.tabs
            currentIndex: strip.currentIndex

            delegate: Rectangle {
                id: tab

                required property int index
                required property string title
                readonly property bool current: index === strip.currentIndex

                width: Math.min(Theme.tabMaxWidth, Math.max(120, list.width / Math.max(1, list.count) - list.spacing))
                height: list.height
                radius: Theme.radius
                z: tabMouse.dragging ? 1 : 0
                color: tabMouse.dragging ? Theme.surfaceHover
                     : current ? Theme.surface : tabMouse.containsMouse ? Theme.surfaceRaised : "transparent"

                Behavior on color { ColorAnimation { duration: Theme.animationMs } }

                TabDragArea {
                    id: tabMouse
                    list: tab.ListView.view
                    index: tab.index
                    onActivated: index => strip.activated(index)
                    onCloseRequested: index => strip.closeRequested(index)
                    onMoveRequested: (from, to) => strip.moveRequested(from, to)
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.spacing * 2
                    anchors.rightMargin: Theme.spacing
                    spacing: Theme.spacing

                    Text {
                        Layout.fillWidth: true
                        text: tab.title.length > 0 ? tab.title : qsTr("New Tab")
                        color: tab.current ? Theme.text : Theme.textMuted
                        font.pixelSize: Theme.fontSize
                        elide: Text.ElideRight
                    }

                    IconButton {
                        Layout.preferredWidth: Theme.tabHeight - Theme.spacing * 2
                        Layout.preferredHeight: Theme.tabHeight - Theme.spacing * 2
                        glyph: "×"
                        tip: qsTr("Close tab")
                        visible: tab.current || tabMouse.containsMouse || hovered
                        onClicked: strip.closeRequested(tab.index)
                    }
                }
            }
        }

        IconButton {
            Layout.alignment: Qt.AlignVCenter
            glyph: "+"
            tip: qsTr("New tab")
            onClicked: strip.newTabRequested()
        }

        IconButton {
            id: menuButton
            Layout.alignment: Qt.AlignVCenter
            glyph: "⋯"
            tip: qsTr("Tabs and sessions")
            onClicked: strip.menuRequested(menuButton)
        }
    }
}
