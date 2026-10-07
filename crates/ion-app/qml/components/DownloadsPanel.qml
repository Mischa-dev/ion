import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// The downloads panel: drops down from the top-right corner when a download
// starts, and on Ctrl+J or the toolbar's download button. Accepts every
// download the profile asks about, into the per-type folder `Downloads`
// picks (the profile's download folder unless configured otherwise).
Popup {
    id: panel

    required property WebEngineProfile profile

    // WebEngineDownloadRequest objects, newest first.
    property var items: []
    readonly property int activeCount: items.filter(d => d.state === WebEngineDownloadRequest.DownloadInProgress).length
    // Average progress of running downloads in 0..1, or -1 if unknown.
    readonly property real activeProgress: {
        let received = 0
        let total = 0
        for (const d of items) {
            if (d.state !== WebEngineDownloadRequest.DownloadInProgress)
                continue
            if (d.totalBytes <= 0)
                return -1
            received += d.receivedBytes
            total += d.totalBytes
        }
        return total > 0 ? received / total : -1
    }

    function track(download) {
        if (!download.isSavePageDownload) {
            const dir = Downloads.targetDirectory(download.downloadFileName, download.mimeType, download.downloadDirectory)
            if (dir !== download.downloadDirectory) {
                download.downloadDirectory = dir
                download.downloadFileName = Downloads.uniqueFileName(dir, download.downloadFileName)
            }
        }
        download.accept()
        items = [download].concat(items)
        open()
    }

    // The profile keeps every download request until it is deleted, so rows
    // we drop also free their request.
    function clearFinished() {
        const finished = items.filter(d => d.isFinished)
        items = items.filter(d => !d.isFinished)
        finished.forEach(d => Downloads.release(d))
    }

    function remove(download) {
        items = items.filter(d => d !== download)
        Downloads.release(download)
        if (items.length === 0)
            close()
    }

    // A click on the toolbar button first closes the panel as an outside
    // press; the exit transition keeps it visible, so the click stays a close.
    function toggle() {
        if (visible)
            close()
        else
            open()
    }

    Connections {
        target: panel.profile
        function onDownloadRequested(download) {
            panel.track(download)
        }
    }

    Shortcut {
        sequences: ["Ctrl+J"]
        onActivated: panel.toggle()
    }
    // The page keeps keyboard focus while the panel is open, so Esc is a
    // shortcut rather than the popup's own close policy.
    Shortcut {
        sequences: ["Esc"]
        enabled: panel.opened
        onActivated: panel.close()
    }

    x: parent.width - width - Theme.spacing * 2
    y: Theme.spacing
    width: Math.min(parent.width - Theme.spacing * 4, Theme.tabMaxWidth * 1.8)
    height: Math.min(parent.height - Theme.spacing * 2, column.implicitHeight + topPadding + bottomPadding)
    padding: Theme.spacing * 2

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
    }

    background: Rectangle {
        radius: Theme.radius
        color: Theme.surface
        border.color: Theme.border
        border.width: 1
    }

    contentItem: ColumnLayout {
        id: column
        spacing: Theme.spacing

        RowLayout {
            Layout.fillWidth: true

            Text {
                Layout.fillWidth: true
                text: qsTr("Downloads")
                color: Theme.text
                font.pixelSize: Theme.fontSize + 1
                font.bold: true
            }
            ToolButton {
                id: clearButton
                visible: panel.items.some(d => d.isFinished)
                focusPolicy: Qt.NoFocus
                contentItem: Text {
                    text: qsTr("Clear")
                    color: clearButton.hovered ? Theme.text : Theme.textMuted
                    font.pixelSize: Theme.fontSize
                }
                background: null
                onClicked: panel.clearFinished()
            }
        }

        Text {
            visible: panel.items.length === 0
            Layout.fillWidth: true
            Layout.topMargin: Theme.spacing
            Layout.bottomMargin: Theme.spacing
            text: qsTr("Nothing downloaded yet")
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            implicitHeight: contentHeight
            visible: count > 0
            clip: true
            spacing: Theme.spacing / 2
            model: panel.items
            boundsBehavior: Flickable.StopAtBounds

            delegate: DownloadsItem {
                required property var modelData
                width: list.width
                download: modelData
                onRemoveRequested: panel.remove(modelData)
            }
        }
    }
}
