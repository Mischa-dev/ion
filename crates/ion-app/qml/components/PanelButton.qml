import QtQuick
import QtQuick.Controls
import Ion

// One entry in the side panel's strip: an icon or letter, highlighted while
// its panel is showing. Right-click asks for its menu.
ToolButton {
    id: control

    // A name from IconSet.qml, else the letter in `glyph`.
    property string iconName
    property string glyph
    // A page whose favicon replaces the glyph once Ion has seen it.
    property string iconPage
    property string tip
    property bool selected: false

    signal menuRequested()

    implicitWidth: Theme.urlBarHeight
    implicitHeight: Theme.urlBarHeight
    focusPolicy: Qt.NoFocus

    contentItem: Item {
        Image {
            id: favicon
            anchors.centerIn: parent
            width: Theme.iconSize
            height: width
            sourceSize: Qt.size(width, height)
            source: control.iconPage.length > 0 ? "image://favicon/" + control.iconPage : ""
            // Unknown icons load as an empty image.
            visible: status === Image.Ready && implicitWidth > 0
        }
        Icon {
            anchors.centerIn: parent
            visible: !favicon.visible && control.iconName.length > 0
            name: control.iconName
            color: control.selected ? Theme.accent : Theme.text
        }
        Text {
            anchors.fill: parent
            visible: !favicon.visible && control.iconName.length === 0
            text: control.glyph
            color: control.selected ? Theme.accent : Theme.text
            font.pixelSize: Theme.fontSize + 2
            font.weight: control.selected ? Font.DemiBold : Font.Normal
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    background: Rectangle {
        radius: Theme.radius
        color: control.selected ? Theme.surfaceRaised
             : control.down ? Theme.surfaceRaised : control.hovered ? Theme.surfaceHover : "transparent"
        border.width: control.selected ? Theme.hairline : 0
        border.color: Theme.border
    }

    TapHandler {
        acceptedButtons: Qt.RightButton
        onTapped: control.menuRequested()
    }

    ToolTip.visible: hovered && tip.length > 0
    ToolTip.text: tip
    ToolTip.delay: 600
}
