import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// One row of the downloads panel: file name, status line, progress, and the
// actions that fit its state. Click a finished download to open it.
Rectangle {
    id: row

    required property var download   // WebEngineDownloadRequest

    signal removeRequested()

    readonly property bool running: download.state === WebEngineDownloadRequest.DownloadInProgress
    readonly property bool completed: download.state === WebEngineDownloadRequest.DownloadCompleted
    readonly property bool interrupted: download.state === WebEngineDownloadRequest.DownloadInterrupted
    readonly property real fraction: Downloads.fraction(download.state, download.receivedBytes, download.totalBytes)

    // Smoothed bytes per second, sampled once a second while running.
    property real rate: 0
    property real lastReceived: 0

    function openFile() {
        Qt.openUrlExternally(Downloads.fileUrl(download.downloadDirectory, download.downloadFileName))
    }

    function showInFolder() {
        Qt.openUrlExternally(Downloads.fileUrl(download.downloadDirectory, ""))
    }

    implicitHeight: content.implicitHeight + Theme.spacing * 2
    radius: Theme.radius
    color: rowMouse.containsMouse && completed ? Theme.surfaceRaised : "transparent"

    Timer {
        interval: 1000
        repeat: true
        running: row.running && !row.download.isPaused
        onRunningChanged: {
            row.lastReceived = row.download.receivedBytes
            row.rate = 0
        }
        onTriggered: {
            const received = row.download.receivedBytes
            row.rate = Downloads.smoothRate(row.rate, received - row.lastReceived)
            row.lastReceived = received
        }
    }

    MouseArea {
        id: rowMouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: row.completed ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: if (row.completed) row.openFile()
    }

    RowLayout {
        id: content
        anchors.fill: parent
        anchors.margins: Theme.spacing
        spacing: Theme.spacing

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing / 2

            Text {
                Layout.fillWidth: true
                text: row.download.downloadFileName
                color: row.completed || row.running ? Theme.text : Theme.textMuted
                font.pixelSize: Theme.fontSize
                elide: Text.ElideMiddle
            }

            Text {
                Layout.fillWidth: true
                text: Downloads.statusText(row.download.state, row.download.isPaused,
                                           row.download.receivedBytes, row.download.totalBytes,
                                           row.rate, row.download.interruptReasonString)
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize - 2
                elide: Text.ElideRight
            }

            // Progress track; indeterminate downloads show a full, dimmed bar.
            Rectangle {
                visible: row.running
                Layout.fillWidth: true
                implicitHeight: 3
                radius: height / 2
                color: Theme.surfaceRaised

                Rectangle {
                    height: parent.height
                    radius: parent.radius
                    width: parent.width * (row.fraction < 0 ? 1 : row.fraction)
                    color: Theme.accent
                    opacity: row.fraction < 0 || row.download.isPaused ? 0.4 : 1

                    Behavior on width { NumberAnimation { duration: Theme.animationMs } }
                }
            }
        }

        IconButton {
            visible: row.running
            glyph: row.download.isPaused ? "▶" : "⏸"
            tip: row.download.isPaused ? qsTr("Resume") : qsTr("Pause")
            onClicked: row.download.isPaused ? row.download.resume() : row.download.pause()
        }
        IconButton {
            // Finished interrupted downloads cannot be resumed.
            visible: row.interrupted && !row.download.isFinished
            glyph: "↻"
            tip: qsTr("Retry")
            onClicked: row.download.resume()
        }
        IconButton {
            visible: row.completed
            glyph: "📁"
            tip: qsTr("Show in folder")
            onClicked: row.showInFolder()
        }
        IconButton {
            glyph: "×"
            tip: row.running ? qsTr("Cancel") : qsTr("Remove from list")
            onClicked: row.running ? row.download.cancel() : row.removeRequested()
        }
    }
}
