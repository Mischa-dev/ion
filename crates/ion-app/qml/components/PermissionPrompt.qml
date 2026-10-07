import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// Ion's own prompt for camera, microphone, location, notifications and the
// other site permissions. Lives inside a BrowserTab, below the toolbar on the
// left like a doorhanger. Requests queue up and are asked one at a time; the
// profile remembers the persistent ones (location, notifications…) on disk.
Rectangle {
    id: prompt

    required property WebEngineView view

    // Pending WebEnginePermission values, oldest first.
    property var queue: []
    readonly property var current: queue.length > 0 ? queue[0] : null
    readonly property string message: current ? Basics.permissionText(current.permissionType, current.origin) : ""
    // Allow stays disabled for a moment after each new request shows, so a
    // click aimed at the page can't grant it by accident. The delay starts
    // again whenever the prompt comes back into view: its tab is shown, or
    // the window is raised or restored.
    property bool armed: false
    readonly property bool onScreen: view.visible && Window.active

    function rearm() {
        armed = false
        armTimer.stop()
        if (current && onScreen)
            armTimer.start()
    }
    onCurrentChanged: rearm()
    onOnScreenChanged: rearm()

    Timer {
        id: armTimer
        interval: Theme.promptArmMs
        onTriggered: prompt.armed = true
    }

    function enqueue(permission) {
        if (Basics.permissionText(permission.permissionType, permission.origin).length === 0) {
            permission.deny()
            return
        }
        queue = queue.concat([permission])
    }

    function answer(allow) {
        if (!current || (allow && !armed))
            return
        if (allow)
            current.grant()
        else
            current.deny()
        queue = queue.slice(1)
        Basics.notifyPermissionsChanged()
    }

    // A new page cancels whatever the old one asked for. The engine drops those
    // requests itself; denying them here would store a "Block" the user never
    // chose for persistent kinds like notifications or location.
    function dropAll() {
        queue = []
    }

    visible: current !== null
    z: 10
    anchors.top: parent.top
    anchors.left: parent.left
    anchors.margins: Theme.spacing * 2
    width: Math.min(parent.width - Theme.spacing * 4, Theme.tabMaxWidth * 1.8)
    height: content.implicitHeight + Theme.spacing * 4
    radius: Theme.radius
    color: Theme.surface
    border.color: Theme.border
    border.width: 1

    Connections {
        target: prompt.view
        function onPermissionRequested(permission) {
            prompt.enqueue(permission)
        }
        function onLoadingChanged(request) {
            if (request.status === WebEngineView.LoadStartedStatus)
                prompt.dropAll()
        }
    }

    ColumnLayout {
        id: content
        anchors.fill: parent
        anchors.margins: Theme.spacing * 2
        spacing: Theme.spacing * 2

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing * 2

            Text {
                text: prompt.current ? Basics.permissionGlyph(prompt.current.permissionType) : ""
                font.pixelSize: Theme.fontSize + 7
                color: Theme.text
            }
            Text {
                Layout.fillWidth: true
                text: prompt.message
                color: Theme.text
                font.pixelSize: Theme.fontSize
                wrapMode: Text.Wrap
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spacing

            DialogButton {
                text: qsTr("Block")
                onClicked: prompt.answer(false)
            }
            DialogButton {
                text: qsTr("Allow")
                primary: true
                enabled: prompt.armed
                onClicked: prompt.answer(true)
            }
        }
    }
}
