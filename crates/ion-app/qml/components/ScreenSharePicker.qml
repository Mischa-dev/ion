import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// Picks what a page may capture once screen sharing is allowed: a whole
// screen or one window. Lives inside a BrowserTab; cancelling (Esc, clicking
// outside, Cancel) tells the page that nothing was shared.
Popup {
    id: picker

    required property WebEngineView view

    property var request: null
    property bool answered: false

    function choose(fromScreens, row) {
        const model = fromScreens ? request.screensModel : request.windowsModel
        if (fromScreens)
            request.selectScreen(model.index(row, 0))
        else
            request.selectWindow(model.index(row, 0))
        answered = true
        close()
    }

    Connections {
        target: picker.view
        function onDesktopMediaRequested(request) {
            picker.request = request
            picker.answered = false
            picker.open()
        }
    }

    onClosed: {
        if (request && !answered)
            request.cancel()
        request = null
    }

    parent: view
    anchors.centerIn: parent
    width: Math.min(view.width - Theme.spacing * 4, Theme.tabMaxWidth * 2)
    height: Math.min(view.height - Theme.spacing * 4, column.implicitHeight + topPadding + bottomPadding)
    padding: Theme.spacing * 2
    modal: true
    focus: true

    background: Rectangle {
        radius: Theme.radius
        color: Theme.surface
        border.color: Theme.border
        border.width: 1
    }

    contentItem: ColumnLayout {
        id: column
        spacing: Theme.spacing

        Text {
            Layout.fillWidth: true
            // "example.com wants to see your screen" (permission type 4).
            text: Basics.permissionText(4, picker.view.url)
            color: Theme.text
            font.pixelSize: Theme.fontSize + 1
            font.bold: true
            wrapMode: Text.Wrap
        }
        Text {
            Layout.fillWidth: true
            text: qsTr("Choose what to share")
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize
        }

        SourceList {
            title: qsTr("Screens")
            model: picker.request ? picker.request.screensModel : null
            fromScreens: true
        }
        SourceList {
            title: qsTr("Windows")
            model: picker.request ? picker.request.windowsModel : null
            fromScreens: false
        }

        Button {
            id: cancelButton
            Layout.alignment: Qt.AlignRight
            text: qsTr("Cancel")
            focusPolicy: Qt.NoFocus
            implicitHeight: Theme.urlBarHeight
            onClicked: picker.close()
            contentItem: Text {
                text: cancelButton.text
                color: Theme.text
                font.pixelSize: Theme.fontSize
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            background: Rectangle {
                radius: Theme.radius
                color: cancelButton.hovered ? Theme.surfaceHover : Theme.surfaceRaised
            }
        }
    }

    component SourceList: ColumnLayout {
        id: section
        property string title
        property var model
        property bool fromScreens

        visible: repeater.count > 0
        Layout.fillWidth: true
        spacing: Theme.spacing / 2

        Text {
            text: section.title
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize - 1
        }

        Repeater {
            id: repeater
            model: section.model

            delegate: ItemDelegate {
                id: source
                required property int index
                // `display` is taken by ItemDelegate itself.
                required property var model

                Layout.fillWidth: true
                implicitHeight: Theme.tabHeight
                onClicked: picker.choose(section.fromScreens, index)

                contentItem: Text {
                    text: source.model.display
                    color: Theme.text
                    font.pixelSize: Theme.fontSize
                    elide: Text.ElideRight
                    verticalAlignment: Text.AlignVCenter
                }
                background: Rectangle {
                    radius: Theme.radius
                    color: source.hovered ? Theme.surfaceHover : "transparent"
                }
            }
        }
    }
}
