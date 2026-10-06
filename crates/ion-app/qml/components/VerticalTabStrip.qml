import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Vertical variant of TabStrip: a sidebar listing tabs top to bottom, for
// people who keep many tabs open. Same model, same signals.
Rectangle {
    id: strip

    required property var tabs   // the Tabs model
    property int currentIndex: 0

    signal activated(int index)
    signal closeRequested(int index)
    signal newTabRequested()
    signal menuRequested(Item anchor)

    implicitWidth: Theme.tabMaxWidth
    color: Theme.background

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
                glyph: "⋯"
                tip: qsTr("Tabs and sessions")
                onClicked: strip.menuRequested(menuButton)
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
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

            delegate: Rectangle {
                id: tab

                required property int index
                required property string title
                required property string url
                readonly property bool current: index === strip.currentIndex

                width: list.width
                height: Theme.tabHeight
                radius: Theme.radius
                color: current ? Theme.surface : tabMouse.containsMouse ? Theme.surfaceRaised : "transparent"

                Behavior on color { ColorAnimation { duration: Theme.animationMs } }

                MouseArea {
                    id: tabMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                    onClicked: mouse => {
                        if (mouse.button === Qt.MiddleButton)
                            strip.closeRequested(tab.index)
                        else
                            strip.activated(tab.index)
                    }
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.spacing * 2
                    anchors.rightMargin: Theme.spacing
                    spacing: Theme.spacing

                    Text {
                        Layout.fillWidth: true
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
                        visible: tab.current || tabMouse.containsMouse || hovered
                        onClicked: strip.closeRequested(tab.index)
                    }
                }
            }
        }
    }
}
