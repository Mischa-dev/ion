import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Reader mode's toolbar, floating at the bottom of the reader page: text
// size, serif or sans, column width, the page's colors, and the way out.
// Settings save to `[reader]` and apply to the open page at once. It fades
// back while you read and comes up again under the pointer.
Rectangle {
    id: bar

    required property var reader   // the tab's ReaderMode

    visible: reader.active
    z: 10
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.bottom: parent.bottom
    anchors.bottomMargin: Theme.spacing * 3
    width: row.implicitWidth + Theme.spacing * 2
    height: Theme.urlBarHeight + Theme.spacing * 2
    radius: height / 2
    color: Theme.surface
    border.color: Theme.border
    border.width: Theme.hairline
    opacity: hover.hovered ? 1 : Theme.dimmedOpacity
    Behavior on opacity { NumberAnimation { duration: Theme.animationMs } }

    HoverHandler { id: hover }

    component Choice: ToolButton {
        id: choice
        property bool selected: false
        property string tip
        implicitHeight: Theme.urlBarHeight
        implicitWidth: Math.max(Theme.urlBarHeight, contentItem.implicitWidth + Theme.spacing * 2)
        focusPolicy: Qt.NoFocus
        contentItem: Text {
            text: choice.text
            font: choice.font
            color: choice.selected ? Theme.accent : Theme.text
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
        background: Rectangle {
            radius: height / 2
            color: choice.down ? Theme.surfaceRaised : choice.hovered ? Theme.surfaceHover : "transparent"
        }
        ToolTip.visible: hovered && tip.length > 0
        ToolTip.text: tip
        ToolTip.delay: 600
    }

    component Separator: Rectangle {
        Layout.preferredWidth: Theme.hairline
        Layout.preferredHeight: Theme.urlBarHeight - Theme.spacing * 2
        color: Theme.border
    }

    RowLayout {
        id: row
        anchors.centerIn: parent
        spacing: Theme.spacing / 2

        Choice {
            text: "A"
            font.pixelSize: Theme.fontSize - 2
            tip: qsTr("Smaller text")
            enabled: bar.reader.fontSize > 12
            onClicked: bar.reader.changeFontSize(-1)
        }
        Choice {
            text: "A"
            font.pixelSize: Theme.fontSize + 4
            tip: qsTr("Larger text")
            enabled: bar.reader.fontSize < 32
            onClicked: bar.reader.changeFontSize(1)
        }

        Separator {}

        Choice {
            text: "Serif"
            font.family: "serif"
            font.pixelSize: Theme.fontSize
            selected: bar.reader.font === "serif"
            onClicked: bar.reader.set("font", "serif")
        }
        Choice {
            text: "Sans"
            font.pixelSize: Theme.fontSize
            selected: bar.reader.font === "sans"
            onClicked: bar.reader.set("font", "sans")
        }

        Separator {}

        Repeater {
            model: [
                { value: "narrow", label: qsTr("Narrow") },
                { value: "medium", label: qsTr("Medium") },
                { value: "wide", label: qsTr("Wide") }
            ]
            // Two lines as far apart as the column is wide.
            delegate: ToolButton {
                id: widthChoice
                required property var modelData
                required property int index
                readonly property bool selected: bar.reader.width === modelData.value
                implicitWidth: Theme.urlBarHeight
                implicitHeight: Theme.urlBarHeight
                focusPolicy: Qt.NoFocus
                onClicked: bar.reader.set("width", modelData.value)
                contentItem: Item {
                    Row {
                        anchors.centerIn: parent
                        spacing: Theme.spacing * (0.6 + 0.7 * widthChoice.index)
                        Repeater {
                            model: 2
                            Rectangle {
                                width: Theme.hairline * 2
                                height: Theme.iconSize - 4
                                radius: Theme.hairline
                                color: widthChoice.selected ? Theme.accent : Theme.text
                            }
                        }
                    }
                }
                background: Rectangle {
                    radius: height / 2
                    color: widthChoice.down ? Theme.surfaceRaised
                         : widthChoice.hovered ? Theme.surfaceHover : "transparent"
                }
                ToolTip.visible: hovered
                ToolTip.text: modelData.label
                ToolTip.delay: 600
            }
        }

        Separator {}

        // Colors: Ion's theme, then light, sepia and dark paper.
        Repeater {
            model: [
                { value: "match", label: qsTr("Theme colors") },
                { value: "light", label: qsTr("Light") },
                { value: "sepia", label: qsTr("Sepia") },
                { value: "dark", label: qsTr("Dark") }
            ]
            delegate: ToolButton {
                id: swatch
                required property var modelData
                readonly property var colors: Reader.themeColors(modelData.value).split(" ")
                readonly property bool selected: bar.reader.theme === modelData.value
                implicitWidth: Theme.urlBarHeight
                implicitHeight: Theme.urlBarHeight
                focusPolicy: Qt.NoFocus
                onClicked: bar.reader.set("theme", modelData.value)
                contentItem: Item {
                    Rectangle {
                        anchors.centerIn: parent
                        width: Theme.iconSize + Theme.hairline * 2
                        height: width
                        radius: width / 2
                        color: swatch.colors.length > 1 ? swatch.colors[0] : Theme.background
                        border.width: swatch.selected ? Theme.hairline * 2 : Theme.hairline
                        border.color: swatch.selected ? Theme.accent : Theme.border

                        Text {
                            anchors.centerIn: parent
                            text: "a"
                            color: swatch.colors.length > 1 ? swatch.colors[1] : Theme.text
                            font.pixelSize: Theme.fontSize - 3
                        }
                    }
                }
                background: Item {}
                ToolTip.visible: hovered
                ToolTip.text: modelData.label
                ToolTip.delay: 600
            }
        }

        Separator {}

        IconButton {
            iconName: "close"
            tip: qsTr("Leave reader mode")
            onClicked: bar.reader.toggle()
        }
    }
}
