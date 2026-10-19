import QtQuick
import QtQuick.Layouts
import QtWebEngine
import Ion

// Shows the web notifications sites send once they are allowed to: as
// desktop notifications when a notification server is running (Linux), so
// they reach you while Ion is in the background; otherwise as toasts that
// stack in the bottom-right corner. Clicking one tells the page it was
// clicked (and brings Ion forward); × or the timeout dismisses it.
ColumnLayout {
    id: toasts

    required property WebEngineProfile profile

    // How many toasts stay on screen at once; older ones are closed.
    readonly property int maxVisible: 3
    property var items: []

    // Desktop notifications on screen, by id. Holding them here also keeps
    // the page's notification objects alive until they go away.
    property var desktop: ({})

    function present(notification) {
        if (DesktopNotifications.available && presentOnDesktop(notification))
            return
        notification.show()
        // A notification with the same tag from the same site replaces the
        // old one instead of stacking beside it.
        const replaces = n => notification.tag.length > 0
            && n.tag === notification.tag && n.origin.toString() === notification.origin.toString()
        for (const old of items.filter(replaces))
            old.close()
        let kept = items.filter(n => !replaces(n)).concat([notification])
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

    // False if the notification server refused it; it becomes a toast.
    function presentOnDesktop(notification) {
        // A notification with the same tag from the same site updates the
        // old one in place.
        let replaces = 0
        for (const id in desktop) {
            const old = desktop[id]
            if (notification.tag.length > 0 && old.tag === notification.tag
                    && old.origin.toString() === notification.origin.toString())
                replaces = Number(id)
        }
        const site = notification.origin.toString().replace(/^[a-z][a-z0-9+.-]*:\/\//i, "")
        const id = DesktopNotifications.show(notification.title, notification.message, site,
                                             notification.icon, replaces)
        if (id === 0)
            return false
        notification.show()
        if (replaces !== 0 && replaces !== id)
            delete desktop[replaces]
        desktop[id] = notification
        // The page closing it takes the desktop one down too.
        notification.closed.connect(() => {
            if (toasts.desktop[id] === notification) {
                delete toasts.desktop[id]
                DesktopNotifications.close(id)
            }
        })
        return true
    }

    Connections {
        target: toasts.profile
        function onPresentNotification(notification) {
            toasts.present(notification)
        }
    }
    Connections {
        target: DesktopNotifications
        function onClicked(id, activationToken) {
            const notification = toasts.desktop[id]
            if (!notification)
                return
            delete toasts.desktop[id]
            const window = toasts.Window.window
            if (window) {
                if (window.visibility === Window.Minimized)
                    window.showNormal()
                Platform.prepareActivation(activationToken)
                window.raise()
                window.requestActivate()
            }
            notification.click()
            notification.close()
        }
        function onClosed(id) {
            const notification = toasts.desktop[id]
            if (!notification)
                return
            delete toasts.desktop[id]
            notification.close()
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
                // Long enough to read a sentence; hovering keeps it open. It
                // waits while Ion is in the background and starts over when
                // the window is active again, so a toast isn't missed.
                interval: Theme.toastMs
                running: !toastMouse.containsMouse && Window.active
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
                        textFormat: Text.PlainText
                        color: Theme.text
                        font.pixelSize: Theme.fontSize
                        font.bold: true
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        visible: text.length > 0
                        text: toast.notification.message
                        textFormat: Text.PlainText
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
                    iconName: "close"
                    tip: qsTr("Dismiss")
                    onClicked: toasts.close(toast.notification)
                }
            }
        }
    }
}
