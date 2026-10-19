import QtQuick
import QtQuick.Layouts
import Ion

// One page in History or Bookmarks: icon, title, address, a note at the end
// (a time, a folder) and a remove button on hover. A click opens it here;
// a middle click or Ctrl+click opens it in a background tab.
Rectangle {
    id: row

    property string url
    property string title
    property string note
    property bool selected: false
    // Shown on hover; empty hides the button.
    property string removeTip: qsTr("Remove")
    // An extra hover action before remove (Bookmarks' edit), empty for none.
    property string editTip: ""

    signal open(bool newTab)
    signal removeRequested()
    signal editRequested()
    signal menuRequested()

    readonly property string host: {
        const match = /^[a-z]+:\/\/(?:www\.)?([^/:?#]+)/i.exec(url)
        return match ? match[1] : url
    }

    implicitHeight: Theme.tabHeight + Theme.spacing * 2
    radius: Theme.radius
    color: selected ? Theme.surfaceRaised : mouse.containsMouse ? Theme.surface : "transparent"

    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
        cursorShape: Qt.PointingHandCursor
        onClicked: mouse => {
            if (mouse.button === Qt.RightButton)
                row.menuRequested()
            else
                row.open(mouse.button === Qt.MiddleButton || (mouse.modifiers & Qt.ControlModifier))
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spacing * 2
        anchors.rightMargin: Theme.spacing * 2
        spacing: Theme.spacing * 2

        // The site's icon once Ion has seen it, else its first letter.
        Item {
            Layout.preferredWidth: Theme.iconSize
            Layout.preferredHeight: Theme.iconSize
            Image {
                id: favicon
                anchors.fill: parent
                sourceSize: Qt.size(width, height)
                source: row.url.length > 0 ? "image://favicon/" + row.url : ""
                visible: status === Image.Ready && implicitWidth > 0
            }
            Rectangle {
                anchors.fill: parent
                visible: !favicon.visible
                radius: width / 2
                color: Theme.surfaceHover
                Text {
                    anchors.centerIn: parent
                    text: row.host.charAt(0).toUpperCase()
                    color: Theme.accent
                    font.pixelSize: Theme.fontSize - 3
                    font.bold: true
                }
            }
        }

        Text {
            Layout.preferredWidth: row.width * 0.5
            Layout.maximumWidth: row.width * 0.5
            text: row.title.length > 0 ? row.title : row.url
            color: Theme.text
            font.pixelSize: Theme.fontSize
            elide: Text.ElideRight
            textFormat: Text.PlainText
        }
        Text {
            Layout.fillWidth: true
            text: row.host
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize - 1
            elide: Text.ElideRight
            textFormat: Text.PlainText
        }
        Text {
            visible: row.note.length > 0
            text: row.note
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize - 1
            font.features: { "tnum": 1 }
            textFormat: Text.PlainText
        }

        IconButton {
            visible: row.editTip.length > 0
            opacity: mouse.containsMouse || hovered ? 1 : 0
            implicitWidth: Theme.tabHeight - Theme.spacing
            implicitHeight: implicitWidth
            glyph: "✎"
            tip: row.editTip
            onClicked: row.editRequested()
        }
        IconButton {
            visible: row.removeTip.length > 0
            opacity: mouse.containsMouse || hovered ? 1 : 0
            implicitWidth: Theme.tabHeight - Theme.spacing
            implicitHeight: implicitWidth
            glyph: "×"
            tip: row.removeTip
            onClicked: row.removeRequested()
        }
    }
}
