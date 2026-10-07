import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Vertical variant of TabStrip: a sidebar listing tabs top to bottom, for
// people who keep many tabs open. Same model, same signals.
//
// Collapsed (`ui.collapseSidebar`), it shows only favicons and expands over the
// page while the pointer rests on it.
Item {
    id: strip

    required property var tabs   // the Tabs model
    property int currentIndex: 0
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

    property bool collapsed: false
    // The pointer has rested on the collapsed sidebar.
    property bool peek: false
    // Showing titles: always when not collapsed, else while peeking.
    readonly property bool expanded: !collapsed || peek

    implicitWidth: collapsed ? Theme.sidebarCollapsedWidth : Theme.tabMaxWidth

    // Rest the pointer on a collapsed sidebar to expand it; leave to collapse.
    Timer {
        id: peekTimer
        interval: Theme.hoverDelayMs
        onTriggered: strip.peek = true
    }

    Rectangle {
        id: panel
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        width: strip.expanded ? Theme.tabMaxWidth : Theme.sidebarCollapsedWidth
        clip: true
        color: Theme.background

        Behavior on width { NumberAnimation { duration: Theme.animationMs; easing.type: Easing.OutCubic } }

        HoverHandler {
            onHoveredChanged: {
                if (hovered && strip.collapsed) {
                    peekTimer.restart()
                } else {
                    peekTimer.stop()
                    strip.peek = false
                }
            }
        }

        // Edge between the expanded sidebar and the page it covers.
        Rectangle {
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.right: parent.right
            width: Theme.hairline
            color: Theme.border
            visible: strip.collapsed && strip.peek
        }

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: Theme.spacing
            spacing: Theme.spacing / 2

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spacing / 2

                IconButton {
                    Layout.fillWidth: true
                    glyph: "+"
                    tip: qsTr("New tab")
                    onClicked: strip.newTabRequested()
                }

                IconButton {
                    id: menuButton
                    visible: strip.expanded
                    glyph: "⋯"
                    tip: qsTr("Tabs and sessions")
                    onClicked: strip.menuRequested(menuButton)
                }

                IconButton {
                    visible: strip.expanded
                    glyph: strip.collapsed ? "»" : "«"
                    tip: strip.collapsed ? qsTr("Keep sidebar open") : qsTr("Collapse sidebar")
                    onClicked: Config.set("ui.collapseSidebar", !strip.collapsed)
                }
            }

            ListView {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: Theme.spacing / 2
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                model: strip.tabs
                currentIndex: strip.currentIndex
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
                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

                delegate: Rectangle {
                    id: tab

                    required property int index
                    required property string title
                    required property string icon
                    required property bool suspended
                    required property string url
                    readonly property bool current: index === strip.currentIndex
                    readonly property var view: {
                        strip.viewsRevision
                        return strip.views ? strip.views.itemAt(index) : null
                    }

                    width: list.width
                    height: Theme.tabHeight
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
                            visible: strip.expanded
                            text: tab.title.length > 0 ? tab.title : tab.url.length > 0 ? tab.url : qsTr("New Tab")
                            color: tab.current ? Theme.text : Theme.textMuted
                            font.pixelSize: Theme.fontSize
                            elide: Text.ElideRight
                        }

                        IconButton {
                            Layout.preferredWidth: Theme.tabHeight - Theme.spacing * 2
                            Layout.preferredHeight: Theme.tabHeight - Theme.spacing * 2
                            glyph: "×"
                            tip: qsTr("Close tab")
                            visible: strip.expanded && (tab.current || tabMouse.containsMouse || hovered)
                            onClicked: strip.closeRequested(tab.index)
                        }
                    }
                }
            }
        }
    }
}
