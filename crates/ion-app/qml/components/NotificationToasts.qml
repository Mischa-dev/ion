import QtQuick
import QtQuick.Layouts
import QtWebEngine
import Ion

// Shows the web notifications sites send once they are allowed to. Toasts
// stack in the bottom-right corner; clicking one tells the page it was
// clicked, × or the timeout dismisses it.
ColumnLayout {
    id: toasts

    required property WebEngineProfile profile

    // How many toasts stay on screen at once; older ones are closed.
    readonly property int maxVisible: 3
    property var items: []

    function present(notification) {
        notification.show()
        let kept = items.concat([notification])
        while (kept.length > maxVisible) {
            kept[0].close()
            kept = kept.slice(1)
        }
        items = kept
    }

    function dismiss(notification) {
        items = items.filter(n => n !== notification)
    }

    // Our own close() does not emit closed(), so drop the toast here too.
    function close(notification) {
        notification.close()
        dismiss(notification)
    }

    Connections {
        target: toasts.profile
        function onPresentNotification(notification) {
            toasts.present(notification)
        }
    }

    anchors.right: parent.right
    anchors.bottom: parent.bottom
    anchors.margins: Theme.spacing * 2
    width: Math.min(parent.width - Theme.spacing * 4, Theme.tabMaxWidth * 1.6)
    spacing: Theme.spacing
    z: 20

    Repeater {
        model: toasts.items

        delegate: Rectangle {
            id: toast
            required property var modelData
            readonly property var notification: modelData

            Layout.fillWidth: true
            implicitHeight: content.implicitHeight + Theme.spacing * 3
            radius: Theme.radius
            color: toastMouse.containsMouse ? Theme.surfaceRaised : Theme.surface
            border.color: Theme.border
            border.width: 1

            // The page closing the notification (or replacing it by tag)
            // removes the toast too.
            Connections {
                target: toast.notification
                function onClosed() { toasts.dismiss(toast.notification) }
            }

            Timer {
                // Long enough to read a sentence; hovering keeps it open.
                interval: Theme.animationMs * 50
                running: !toastMouse.containsMouse
                onTriggered: toasts.close(toast.notification)
            }

            MouseArea {
                id: toastMouse
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    toast.notification.click()
                    toasts.close(toast.notification)
                }
            }

            RowLayout {
                id: content
                anchors.fill: parent
                anchors.margins: Theme.spacing * 1.5
                spacing: Theme.spacing

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacing / 2

                    Text {
                        Layout.fillWidth: true
                        text: toast.notification.title
                        color: Theme.text
                        font.pixelSize: Theme.fontSize
                        font.bold: true
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        visible: text.length > 0
                        text: toast.notification.message
                        color: Theme.text
                        font.pixelSize: Theme.fontSize - 1
                        wrapMode: Text.Wrap
                        maximumLineCount: 3
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        text: toast.notification.origin.toString()
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSize - 2
                        elide: Text.ElideRight
                    }
                }

                IconButton {
                    Layout.alignment: Qt.AlignTop
                    glyph: "×"
                    tip: qsTr("Dismiss")
                    onClicked: toasts.close(toast.notification)
                }
            }
        }
    }
}
