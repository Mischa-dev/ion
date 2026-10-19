import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Toolbar button for whatever tabs are playing: a note, lit while something
// plays. Its panel lists each tab with media, with play/pause,
// previous/next where the site offers them, picture-in-picture for videos,
// and a click on the row to go to the tab.
IconButton {
    id: button

    property MediaSessions sessions   // may be null

    readonly property var tabs: sessions?.tabs ?? []
    readonly property bool playing: sessions?.anyPlaying ?? false

    visible: tabs.length > 0
    iconName: "music"
    active: playing
    tip: playing ? qsTr("Playing in %1").arg(sessions.host(sessions.active.url.toString()))
                 : qsTr("Media")
    onClicked: panel.opened ? panel.close() : panel.open()

    Popup {
        id: panel
        y: button.height + Theme.spacing / 2
        x: button.width - width
        width: Theme.tabMaxWidth + Theme.spacing * 22
        padding: Theme.spacing
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent

        background: Rectangle {
            radius: Theme.radius
            color: Theme.surface
            border.width: Theme.hairline
            border.color: Theme.border
        }
        enter: Transition {
            NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
        }
        exit: Transition {
            NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
        }

        // Positions move on between the pages' reports.
        property real now: Date.now()
        Timer {
            interval: 1000
            repeat: true
            running: panel.opened && button.playing
            onTriggered: panel.now = Date.now()
        }

        // Nothing left to show.
        Connections {
            target: button
            function onTabsChanged() { if (button.tabs.length === 0) panel.close() }
        }

        contentItem: ColumnLayout {
            spacing: Theme.spacing / 2

            Repeater {
                model: button.tabs

                delegate: ItemDelegate {
                    id: row

                    required property var modelData
                    readonly property var view: modelData
                    readonly property var media: view.media ?? ({})
                    readonly property real reportedAt: {
                        media
                        return Date.now()
                    }
                    readonly property real position: media.duration > 0
                        ? Math.min(media.duration, (media.position ?? 0)
                                   + (media.playing ? Math.max(0, panel.now - reportedAt) / 1000 : 0))
                        : 0

                    function clock(seconds) {
                        const s = Math.floor(seconds)
                        const hours = Math.floor(s / 3600)
                        const minutes = Math.floor(s / 60) % 60
                        const rest = String(s % 60).padStart(2, "0")
                        return hours > 0 ? hours + ":" + String(minutes).padStart(2, "0") + ":" + rest
                                         : minutes + ":" + rest
                    }

                    Layout.fillWidth: true
                    padding: Theme.spacing
                    hoverEnabled: true
                    focusPolicy: Qt.NoFocus
                    onClicked: {
                        panel.close()
                        button.sessions.show(view)
                    }

                    background: Rectangle {
                        radius: Theme.radius
                        color: row.hovered ? Theme.surfaceHover : "transparent"
                    }

                    contentItem: ColumnLayout {
                        spacing: Theme.spacing

                        RowLayout {
                            spacing: Theme.spacing * 1.5

                            // The track's artwork, else the site's icon.
                            Rectangle {
                                Layout.preferredWidth: Theme.urlBarHeight + Theme.spacing * 2
                                Layout.preferredHeight: Layout.preferredWidth
                                radius: Theme.radius
                                color: Theme.surfaceRaised
                                clip: true

                                Image {
                                    id: art
                                    anchors.fill: parent
                                    source: row.media.artwork ?? ""
                                    fillMode: Image.PreserveAspectCrop
                                    asynchronous: true
                                    sourceSize.width: width * 2
                                    visible: status === Image.Ready
                                }
                                Image {
                                    id: favicon
                                    anchors.centerIn: parent
                                    width: Theme.iconSize
                                    height: Theme.iconSize
                                    source: art.visible ? "" : row.view.icon
                                    sourceSize.width: Theme.iconSize * 2
                                    visible: !art.visible && status === Image.Ready
                                }
                                Icon {
                                    anchors.centerIn: parent
                                    name: "music"
                                    color: Theme.textMuted
                                    visible: !art.visible && !favicon.visible
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0

                                Label {
                                    Layout.fillWidth: true
                                    text: row.media.title || row.view.title || button.sessions.host(row.view.url.toString())
                                    color: Theme.text
                                    font.pixelSize: Theme.fontSize
                                    font.weight: Font.Medium
                                    elide: Text.ElideRight
                                }
                                Label {
                                    Layout.fillWidth: true
                                    text: row.media.artist || button.sessions.host(row.view.url.toString())
                                    color: Theme.textMuted
                                    font.pixelSize: Theme.fontSize - 1
                                    elide: Text.ElideRight
                                }
                            }

                            IconButton {
                                visible: row.media.canPrevious ?? false
                                iconName: "skip-back"
                                tip: qsTr("Previous")
                                onClicked: row.view.mediaAction("previous")
                            }
                            IconButton {
                                iconName: row.media.playing ? "pause" : "play"
                                tip: row.media.playing ? qsTr("Pause") : qsTr("Play")
                                onClicked: row.view.mediaAction("playpause")
                            }
                            IconButton {
                                visible: row.media.canNext ?? false
                                iconName: "skip-forward"
                                tip: qsTr("Next")
                                onClicked: row.view.mediaAction("next")
                            }
                            IconButton {
                                visible: (row.media.hasVideo ?? false) && button.sessions.pip !== null
                                iconName: "pip"
                                tip: row.view.pictureInPicture ? qsTr("Back to the tab")
                                                               : qsTr("Picture-in-picture")
                                onClicked: {
                                    panel.close()
                                    if (row.view.pictureInPicture)
                                        button.sessions.pip.restore(false)
                                    else
                                        button.sessions.pip.open(row.view, false)
                                }
                            }
                        }

                        // Progress, for media with a length.
                        RowLayout {
                            visible: (row.media.duration ?? 0) > 0
                            spacing: Theme.spacing

                            Label {
                                text: row.clock(row.position)
                                color: Theme.textMuted
                                font.pixelSize: Theme.fontSize - 2
                                font.features: { "tnum": 1 }
                            }
                            Rectangle {
                                Layout.fillWidth: true
                                height: Theme.hairline * 3
                                radius: height / 2
                                color: Theme.surfaceHover

                                Rectangle {
                                    width: parent.width * (row.media.duration > 0 ? row.position / row.media.duration : 0)
                                    height: parent.height
                                    radius: parent.radius
                                    color: Theme.accent
                                }
                                // Click to jump.
                                MouseArea {
                                    anchors.fill: parent
                                    anchors.margins: -Theme.spacing
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: mouse => row.view.mediaAction("seekto",
                                        row.media.duration * Math.max(0, Math.min(1, (mouse.x - Theme.spacing) / parent.width)))
                                }
                            }
                            Label {
                                text: row.clock(row.media.duration ?? 0)
                                color: Theme.textMuted
                                font.pixelSize: Theme.fontSize - 2
                                font.features: { "tnum": 1 }
                            }
                        }
                    }
                }
            }
        }
    }
}
