import QtQuick
import QtQuick.Layouts
import QtWebEngine
import Ion

// Asks before a page opens another app through a link (`mailto:`, Zoom,
// Slack…), with "always allow" per site and scheme; when a site wants to
// handle a kind of link itself (`navigator.registerProtocolHandler`); and
// when a site wants to keep using a file or folder picked for it (File
// System Access: vscode.dev, Photopea).
// Lives inside a BrowserTab, below the toolbar on the left like the
// permission prompt. One question at a time; a new page drops it.
Rectangle {
    id: prompt

    required property WebEngineView view

    // { kind: "open", url, origin, text, always } or
    // { kind: "handler" | "files", request, text }.
    property var current: null
    // Allow waits a moment after the prompt shows, so a click aimed at the
    // page can't answer it.
    property bool armed: false

    // A page at `origin` wants to open `url` in another app.
    function ask(url, origin) {
        if (Protocols.allows(origin, url)) {
            Qt.openUrlExternally(url)
            return
        }
        // One question at a time; a page can't stack them up.
        if (current)
            return
        const lines = Protocols.openPrompt(origin, url)
        alwaysBox.checked = false
        current = { kind: "open", url: url, origin: origin, text: lines[0], always: lines[1] }
    }

    function askHandler(request) {
        if (current) {
            request.reject()
            return
        }
        current = {
            kind: "handler",
            request: request,
            text: Protocols.handlerPrompt(request.origin.toString(), request.scheme)
        }
    }

    function askFiles(request) {
        if (current) {
            request.reject()
            return
        }
        current = {
            kind: "files",
            request: request,
            text: Protocols.fileAccessPrompt(request.origin.toString(), request.filePath.toString(),
                request.handleType === WebEngineFileSystemAccessRequest.Directory,
                (request.accessFlags & WebEngineFileSystemAccessRequest.Write) !== 0)
        }
    }

    function answer(allow) {
        if (!current || (allow && !armed))
            return
        const asked = current
        current = null
        if (asked.kind === "files") {
            if (allow)
                asked.request.accept()
            else
                asked.request.reject()
            return
        }
        if (asked.kind === "handler") {
            if (allow) {
                asked.request.accept()
                Protocols.setHandler(asked.request.scheme, asked.request.origin.toString())
            } else {
                asked.request.reject()
            }
            return
        }
        if (!allow)
            return
        if (alwaysBox.checked)
            Protocols.allow(asked.origin, asked.url)
        Qt.openUrlExternally(asked.url)
    }

    onCurrentChanged: {
        armed = false
        if (current)
            armTimer.restart()
    }
    Timer {
        id: armTimer
        interval: Theme.promptArmMs
        onTriggered: prompt.armed = true
    }

    Connections {
        target: prompt.view
        function onRegisterProtocolHandlerRequested(request) {
            prompt.askHandler(request)
        }
        function onFileSystemAccessRequested(request) {
            prompt.askFiles(request)
        }
        function onLoadingChanged(info) {
            if (info.status === WebEngineView.LoadStartedStatus && prompt.current
                    && prompt.current.kind !== "open") {
                prompt.current.request.reject()
                prompt.current = null
            }
        }
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

    ColumnLayout {
        id: content
        anchors.fill: parent
        anchors.margins: Theme.spacing * 2
        spacing: Theme.spacing * 2

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing * 2

            Text {
                text: prompt.current && prompt.current.kind === "files" ? "📁" : "↗"
                font.pixelSize: Theme.fontSize + 7
                color: Theme.text
            }
            Text {
                Layout.fillWidth: true
                text: prompt.current ? prompt.current.text : ""
                color: Theme.text
                font.pixelSize: Theme.fontSize
                wrapMode: Text.Wrap
            }
        }
        IonCheckBox {
            id: alwaysBox
            Layout.fillWidth: true
            visible: prompt.current !== null && prompt.current.kind === "open"
            text: visible ? prompt.current.always : ""
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spacing

            DialogButton {
                text: prompt.current && prompt.current.kind !== "open" ? qsTr("Block") : qsTr("Cancel")
                onClicked: prompt.answer(false)
            }
            DialogButton {
                text: prompt.current && prompt.current.kind !== "open" ? qsTr("Allow") : qsTr("Open")
                primary: true
                enabled: prompt.armed
                onClicked: prompt.answer(true)
            }
        }
    }
}
