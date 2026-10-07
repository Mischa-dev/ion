import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Horizontal tab strip. Reads titles from the shared tab model and reports
// clicks; Main.qml owns the model and the views.
Rectangle {
    id: strip

    required property ListModel tabs
    property int currentIndex: 0

    signal activated(int index)
    signal closeRequested(int index)
    signal newTabRequested()

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
    }
}
