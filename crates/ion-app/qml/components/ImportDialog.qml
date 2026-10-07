import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// "Import bookmarks and history": lists the other browsers found on this
// machine and pulls one's bookmarks and history into Ion.
Popup {
    id: root

    property var sources: []
    property string result: ""

    function show() {
        sources = Bookmarks.importSources()
        result = ""
        open()
    }

    function importFrom(browser) {
        const bookmarks = Bookmarks.importFrom(browser)
        const pages = History.importFrom(browser)
        if (bookmarks < 0 && pages < 0) {
            result = qsTr("Couldn't read %1's profile. Try again with %1 closed.").arg(browser)
            return
        }
        History.save()
        result = qsTr("Imported %1 bookmarks and %2 history pages from %3.")
            .arg(Math.max(bookmarks, 0)).arg(Math.max(pages, 0)).arg(browser)
    }

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Theme.urlBarHeight * 3
    width: Math.min(Theme.paletteWidth, parent.width - Theme.spacing * 4)
    padding: Theme.spacing * 2
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    Overlay.modal: Rectangle {
        color: Qt.rgba(Theme.background.r, Theme.background.g, Theme.background.b, Theme.scrimOpacity)
    }

    background: Rectangle {
        radius: Theme.radius
        color: Theme.surface
        border.width: 1
        border.color: Theme.border
    }

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
    }

    // Opened from the palette, which hands focus back as it closes; take it
    // afterwards so Escape reaches this popup.
    onOpened: Qt.callLater(() => content.forceActiveFocus())

    contentItem: ColumnLayout {
        id: content
        spacing: Theme.spacing
        focus: true
        Keys.onEscapePressed: root.close()

        Text {
            Layout.fillWidth: true
            text: qsTr("Import bookmarks and history")
            color: Theme.text
            font.pixelSize: Theme.fontSize + 2
            font.bold: true
        }

        Text {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize
            text: root.sources.length > 0
                ? qsTr("Bookmarks land in an “Imported from …” folder. Pages you already have are kept.")
                : qsTr("No Chrome, Chromium, Brave, Vivaldi, Edge or Firefox profile was found.")
        }

        Repeater {
            model: root.sources

            delegate: Rectangle {
                id: row

                required property string modelData

                Layout.fillWidth: true
                implicitHeight: Theme.tabHeight + Theme.spacing * 2
                radius: Theme.radius
                color: mouse.containsMouse ? Theme.surfaceHover : Theme.surfaceRaised

                Text {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.spacing * 2
                    verticalAlignment: Text.AlignVCenter
                    text: qsTr("Import from %1").arg(row.modelData)
                    color: Theme.text
                    font.pixelSize: Theme.fontSize
                }

                MouseArea {
                    id: mouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.importFrom(row.modelData)
                }
            }
        }

        Text {
            Layout.fillWidth: true
            visible: root.result.length > 0
            wrapMode: Text.Wrap
            text: root.result
            color: Theme.text
            font.pixelSize: Theme.fontSize
        }
    }
}
