import QtQuick
import QtQuick.Layouts
import QtWebEngine
import Ion

// The new-tab page: a clock, a greeting and site shortcuts, drawn in Ion's
// theme over an empty tab. Lives inside a BrowserTab and hides as soon as the
// tab navigates somewhere.
Rectangle {
    id: page

    required property WebEngineView view

    property date now: new Date()

    visible: Basics.isNewTabUrl(view.url) && !view.loading
    anchors.fill: parent
    z: 5
    color: Theme.background

    Timer {
        interval: 1000
        repeat: true
        running: page.visible
        triggeredOnStart: true
        onTriggered: page.now = new Date()
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - Theme.spacing * 8, Theme.tabMaxWidth * 3)
        spacing: Theme.spacing * 2

        Text {
            Layout.alignment: Qt.AlignHCenter
            // Hours and minutes in the locale's style, without seconds.
            text: Qt.formatTime(page.now, Qt.locale().timeFormat(Locale.ShortFormat).replace(/[:.]ss/, ""))
            color: Theme.text
            font.pixelSize: Theme.fontSize * 5
            font.weight: Font.Light
        }

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: Basics.greeting(page.now.getHours())
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize + 5
        }

        GridLayout {
            id: grid
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: Theme.spacing * 6
            columns: Math.max(1, Math.min(Basics.shortcutCount, Math.floor(parent.width / (tileSize + columnSpacing))))
            columnSpacing: Theme.spacing * 2
            rowSpacing: Theme.spacing * 2

            readonly property int tileSize: Theme.tabHeight * 3

            Repeater {
                model: Basics.shortcutCount

                delegate: Rectangle {
                    id: tile
                    required property int index

                    Layout.preferredWidth: grid.tileSize
                    Layout.preferredHeight: grid.tileSize
                    radius: Theme.radius * 1.5
                    color: tileMouse.containsMouse ? Theme.surfaceRaised : Theme.surface

                    Behavior on color { ColorAnimation { duration: Theme.animationMs } }

                    ColumnLayout {
                        anchors.centerIn: parent
                        width: parent.width - Theme.spacing * 2
                        spacing: Theme.spacing

                        Rectangle {
                            Layout.alignment: Qt.AlignHCenter
                            implicitWidth: Theme.tabHeight * 1.25
                            implicitHeight: implicitWidth
                            radius: width / 2
                            color: Theme.surfaceHover

                            Text {
                                anchors.centerIn: parent
                                text: Basics.shortcutLetter(tile.index)
                                color: Theme.accent
                                font.pixelSize: Theme.fontSize + 5
                                font.bold: true
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            text: Basics.shortcutTitle(tile.index)
                            color: Theme.text
                            font.pixelSize: Theme.fontSize - 1
                            horizontalAlignment: Text.AlignHCenter
                            elide: Text.ElideRight
                        }
                    }

                    MouseArea {
                        id: tileMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: page.view.url = Basics.shortcutUrl(tile.index)
                    }
                }
            }
        }
    }
}
