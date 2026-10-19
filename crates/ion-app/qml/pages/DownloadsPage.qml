import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// ion://downloads: running downloads, then every past one (kept across
// restarts by `DownloadHistory`), newest first, grouped by day.
LibraryFrame {
    id: page


    readonly property var running: (browser && browser.downloadItems ? browser.downloadItems : [])
        .filter(d => !d.isFinished)
    property var past: []

    readonly property var glyphs: ({
        document: "▤", image: "▣", audio: "♪", video: "▶", archive: "▦", other: "▢"
    })

    function refresh() {
        past = JSON.parse(DownloadHistory.list(query))
    }

    function dayLabel(seconds) {
        const date = new Date(seconds * 1000)
        const today = new Date()
        today.setHours(0, 0, 0, 0)
        const day = new Date(date)
        day.setHours(0, 0, 0, 0)
        const days = Math.round((today - day) / 86400000)
        if (days === 0)
            return qsTr("Today")
        if (days === 1)
            return qsTr("Yesterday")
        return date.toLocaleDateString(Qt.locale(), date.getFullYear() === today.getFullYear()
                                       ? "d MMMM" : "d MMMM yyyy")
    }

    function openFile(item) {
        if (item.exists)
            Qt.openUrlExternally(Downloads.fileUrl(item.directory, item.fileName))
    }

    function showInFolder(item) {
        Qt.openUrlExternally(Downloads.fileUrl(item.directory, ""))
    }

    current: "downloads"
    title: qsTr("Downloads")
    placeholder: qsTr("Search downloads")
    onQueryChanged: refresh()
    Component.onCompleted: refresh()
    // A file deleted outside Ion shows as deleted when the page is shown again.
    onVisibleChanged: if (visible) refresh()

    Connections {
        target: DownloadHistory
        function onRevisionChanged() { page.refresh() }
    }

    actions: [
        DialogButton {
            text: qsTr("Clear list")
            visible: page.past.length > 0
            onClicked: DownloadHistory.clear()
        }
    ]

    Flickable {
        id: scroller
        anchors.fill: parent
        contentHeight: column.implicitHeight + Theme.spacing * 8
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {}

        ColumnLayout {
            id: column
            x: Math.max(Theme.spacing * 4, (scroller.width - width) / 2)
            width: Math.min(scroller.width - Theme.spacing * 8, Theme.readableWidth + Theme.tabMaxWidth)
            spacing: Theme.hairline

            Text {
                visible: page.running.length > 0
                Layout.topMargin: Theme.spacing * 2
                Layout.leftMargin: Theme.spacing * 2
                Layout.bottomMargin: Theme.spacing
                text: qsTr("In progress")
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize - 1
                font.weight: Font.DemiBold
            }
            Repeater {
                model: page.running
                delegate: DownloadsItem {
                    required property var modelData
                    Layout.fillWidth: true
                    download: modelData
                }
            }

            Repeater {
                model: page.past
                delegate: ColumnLayout {
                    id: entry
                    required property var modelData
                    required property int index
                    readonly property string day: page.dayLabel(modelData.finished)
                    readonly property bool firstOfDay: index === 0 || page.dayLabel(page.past[index - 1].finished) !== day
                    readonly property bool usable: modelData.exists && modelData.outcome === "completed"

                    Layout.fillWidth: true
                    spacing: 0

                    Text {
                        visible: entry.firstOfDay
                        Layout.topMargin: Theme.spacing * 4
                        Layout.leftMargin: Theme.spacing * 2
                        Layout.bottomMargin: Theme.spacing
                        text: entry.day
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSize - 1
                        font.weight: Font.DemiBold
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: rowLayout.implicitHeight + Theme.spacing * 3
                        radius: Theme.radius
                        color: rowMouse.containsMouse ? Theme.surface : "transparent"

                        MouseArea {
                            id: rowMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            cursorShape: entry.usable ? Qt.PointingHandCursor : Qt.ArrowCursor
                            onClicked: mouse => {
                                if (mouse.button === Qt.RightButton) {
                                    menu.item = entry.modelData
                                    menu.popup()
                                } else {
                                    page.openFile(entry.modelData)
                                }
                            }
                        }

                        RowLayout {
                            id: rowLayout
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.leftMargin: Theme.spacing * 2
                            anchors.rightMargin: Theme.spacing * 2
                            spacing: Theme.spacing * 2

                            Rectangle {
                                implicitWidth: Theme.tabHeight
                                implicitHeight: Theme.tabHeight
                                radius: Theme.radius
                                color: Theme.surfaceRaised
                                Text {
                                    anchors.centerIn: parent
                                    text: page.glyphs[entry.modelData.category] || page.glyphs.other
                                    color: entry.usable ? Theme.accent : Theme.textMuted
                                    font.pixelSize: Theme.fontSize + 2
                                }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: Theme.hairline
                                Text {
                                    Layout.fillWidth: true
                                    text: entry.modelData.fileName
                                    color: entry.usable ? Theme.text : Theme.textMuted
                                    font.pixelSize: Theme.fontSize
                                    font.strikeout: !entry.modelData.exists && entry.modelData.outcome === "completed"
                                    elide: Text.ElideMiddle
                                    textFormat: Text.PlainText
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: [entry.modelData.outcome === "cancelled" ? qsTr("Cancelled")
                                           : entry.modelData.outcome === "failed" ? qsTr("Failed")
                                           : !entry.modelData.exists ? qsTr("Deleted") : entry.modelData.size,
                                           entry.modelData.host].filter(s => s.length > 0).join("  ·  ")
                                    color: entry.modelData.outcome === "failed" ? Theme.danger : Theme.textMuted
                                    font.pixelSize: Theme.fontSize - 1
                                    elide: Text.ElideRight
                                    textFormat: Text.PlainText
                                }
                            }
                            DialogButton {
                                visible: rowMouse.containsMouse || hovered
                                text: entry.usable ? qsTr("Show in folder") : qsTr("Download again")
                                onClicked: entry.usable ? page.showInFolder(entry.modelData)
                                                        : page.browser.openTab(entry.modelData.url, false)
                            }
                            IconButton {
                                opacity: rowMouse.containsMouse || hovered ? 1 : 0
                                glyph: "×"
                                tip: qsTr("Remove from list")
                                onClicked: DownloadHistory.remove(entry.modelData.id)
                            }
                        }
                    }
                }
            }
        }
    }

    Text {
        anchors.centerIn: parent
        visible: page.past.length === 0 && page.running.length === 0
        text: page.query.length > 0 ? qsTr("No downloads match “%1”").arg(page.query)
                                    : qsTr("Files you download show up here")
        color: Theme.textMuted
        font.pixelSize: Theme.fontSize + 2
    }

    IonMenu {
        id: menu
        property var item: ({})
        IonMenuItem {
            text: qsTr("Open")
            enabled: menu.item.exists === true
            onTriggered: page.openFile(menu.item)
        }
        IonMenuItem {
            text: qsTr("Show in folder")
            onTriggered: page.showInFolder(menu.item)
        }
        IonMenuItem {
            text: qsTr("Copy download link")
            onTriggered: {
                clipboard.text = menu.item.url
                clipboard.selectAll()
                clipboard.copy()
            }
        }
        IonMenuItem {
            text: qsTr("Download again")
            onTriggered: page.browser.openTab(menu.item.url, false)
        }
        IonMenuSeparator {}
        IonMenuItem {
            text: qsTr("Remove from list")
            onTriggered: DownloadHistory.remove(menu.item.id)
        }
    }

    TextEdit { id: clipboard; visible: false }
}
