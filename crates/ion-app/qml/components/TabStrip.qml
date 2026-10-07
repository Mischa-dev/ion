import QtQuick
import QtQuick.Controls
import QtQml.Models
import QtQuick.Layouts
import Ion

// Horizontal tab strip over the `Tabs` model. Reports clicks; Main.qml decides
// what they do. VerticalTabStrip.qml is the sidebar variant.
Rectangle {
    id: strip

    required property var tabs   // the Tabs model
    property int currentIndex: 0
    // Only this workspace's tabs are listed (`Tabs.workspace`).
    property int workspace: 0
    // The Repeater of BrowserTabs in Main.qml, for favicons and load state, and
    // a counter bumped whenever it gains or loses a view.
    property var views: null
    property int viewsRevision: 0

    signal activated(int index)
    signal closeRequested(int index)
    signal moveRequested(int from, int to)
    signal newTabRequested()
    signal menuRequested(Item anchor)
    signal tabMenuRequested(int index)
    signal workspaceMenuRequested(Item anchor)

    // Position in the list of the tab at `row` of the model, or -1.
    function shownIndex(row) {
        return shown.mapFromSource(strip.tabs.index(row, 0)).row
    }

    implicitHeight: Theme.tabHeight + Theme.spacing
    color: Theme.background

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spacing
        anchors.rightMargin: Theme.spacing
        anchors.topMargin: Theme.spacing
        spacing: Theme.spacing / 2

        WorkspaceButton {
            Layout.alignment: Qt.AlignVCenter
            Layout.maximumWidth: Theme.tabMaxWidth / 2
            onMenuRequested: anchor => strip.workspaceMenuRequested(anchor)
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: ListView.Horizontal
            spacing: Theme.spacing / 2
            clip: true
            interactive: contentWidth > width
            // Rows keep their model index in `row`; the list's own indexes
            // only count this workspace's tabs.
            model: SortFilterProxyModel {
                id: shown
                model: strip.tabs
                filters: ValueFilter { roleName: "workspace"; value: strip.workspace }
            }
            currentIndex: {
                list.count
                return strip.shownIndex(strip.currentIndex)
            }
            highlightFollowsCurrentItem: false
            // Keep the current tab fully in view when it changes or tabs are added.
            onCurrentIndexChanged: Qt.callLater(() => list.positionViewAtIndex(list.currentIndex, ListView.Contain))
            onCountChanged: Qt.callLater(() => list.positionViewAtIndex(list.currentIndex, ListView.Contain))

            add: Transition {
                NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationSlowMs; easing.type: Easing.OutCubic }
            }
            remove: Transition {
                NumberAnimation { property: "opacity"; to: 0; duration: Theme.animationMs }
            }

            delegate: Rectangle {
                id: tab

                required property int row
                required property string title
                required property string icon
                required property bool suspended
                readonly property bool current: row === strip.currentIndex
                readonly property var view: {
                    strip.viewsRevision
                    return strip.views ? strip.views.itemAt(row) : null
                }

                width: Math.min(Theme.tabMaxWidth, Math.max(Theme.tabMinWidth, list.width / Math.max(1, list.count) - list.spacing))
                height: list.height
                radius: Theme.radius
                z: tabMouse.dragging ? 1 : 0
                color: tabMouse.dragging ? Theme.surfaceHover
                     : current ? Theme.surface : tabMouse.containsMouse ? Theme.surfaceRaised : "transparent"

                Behavior on color { ColorAnimation { duration: Theme.animationMs } }

                TabDragArea {
                    id: tabMouse
                    list: tab.ListView.view
                    index: tab.row
                    onActivated: index => strip.activated(index)
                    onCloseRequested: index => strip.closeRequested(index)
                    onMoveRequested: (from, to) => strip.moveRequested(from, to)
                    onMenuRequested: index => strip.tabMenuRequested(index)
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
                        onClicked: strip.closeRequested(tab.row)
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
