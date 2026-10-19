import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import QtWebEngine
import Ion

// Picture-in-picture. QtWebEngine has no window of its own for it, so Ion
// moves the tab's whole web view into this small window and has the page
// make the video fill it (`Media.pip*` scripts, in Ion's script world).
// Hovering shows controls; dragging moves the window, its edges resize it.
//
// The window is titled "Picture-in-Picture", like Firefox's, so window
// manager rules that float and pin that title apply to Ion's too.
Window {
    id: pip

    // Where tab views live in the browser window, to put the view back.
    required property Item home
    // The browser window, raised by "Back to tab".
    required property Window browser

    // The tab in picture-in-picture, or null.
    property var view: null
    // Opened by switching away from the tab, so returning to it closes it.
    property bool automatic: false
    // The frame holding the video, for the controls.
    property var videoFrame: null
    property var videoState: ({ paused: false, muted: false, time: 0, duration: 0 })
    // Width people last gave the window, kept while Ion runs.
    property int preferredWidth: 480

    readonly property bool active: view !== null

    title: "Picture-in-Picture"
    flags: Qt.Window | Qt.FramelessWindowHint | Qt.WindowStaysOnTopHint
    color: Theme.videoCanvas
    minimumWidth: 200
    minimumHeight: 112
    width: preferredWidth
    height: Math.round(preferredWidth * 9 / 16)
    visible: false

    // Every frame of `frame`'s tree, each with the frames above it.
    function framesUnder(frame, parents, out) {
        out.push({ frame: frame, parents: parents })
        for (const child of frame.children)
            framesUnder(child, parents.concat([frame]), out)
        return out
    }

    // Pop `tab`'s video out. `auto` when switching tabs did it; then a tab
    // without a playing video stays put, silently.
    function open(tab, auto) {
        if (!tab || tab === view)
            return
        if (view)
            restore(false)
        const frames = framesUnder(tab.mainFrame, [], [])
        let pending = frames.length
        let best = null
        for (const entry of frames) {
            entry.frame.runJavaScript(Media.pipPickScript, WebEngineScript.ApplicationWorld, result => {
                const found = result ? JSON.parse(result) : null
                if (found && (!best || found.score > best.score))
                    best = { score: found.score, playing: found.playing, entry: entry }
                if (--pending === 0)
                    pip.enter(tab, best, auto)
            })
        }
    }

    function enter(tab, best, auto) {
        if (!best || (auto && !best.playing)) {
            if (!auto)
                tab.notify(qsTr("No video on this page"))
            return
        }
        const entry = best.entry
        entry.frame.runJavaScript(Media.pipEnterScript, WebEngineScript.ApplicationWorld, size => {
            if (!size) {
                if (!auto)
                    tab.notify(qsTr("No video on this page"))
                return
            }
            // Each frame above lets its iframe fill the view, top-down.
            let child = entry.frame
            for (let i = entry.parents.length - 1; i >= 0; i--) {
                entry.parents[i].runJavaScript(Media.pipFrameScript(child.url.toString()),
                                               WebEngineScript.ApplicationWorld)
                child = entry.parents[i]
            }
            const parts = size.split("x")
            pip.videoFrame = entry.frame
            pip.automatic = auto
            pip.view = tab
            tab.pictureInPicture = true
            tab.parent = content
            pip.width = pip.preferredWidth
            pip.height = Media.pipHeight(Number(parts[0]), Number(parts[1]), pip.preferredWidth)
            if (!pip.visible) {
                const screen = pip.browser.screen
                pip.x = screen.virtualX + screen.desktopAvailableWidth - pip.width - Theme.spacing * 6
                pip.y = screen.virtualY + screen.desktopAvailableHeight - pip.height - Theme.spacing * 6
            }
            pip.show()
            pip.raise()
            pip.refresh()
        })
    }

    // Put the video back in its tab. `pause` stops it too (closing the
    // window, as in other browsers).
    function restore(pause) {
        const tab = view
        if (!tab)
            return
        if (pause)
            control("pause")
        runInFrames(tab.mainFrame, Media.pipExitScript)
        tab.parent = home
        tab.pictureInPicture = false
        view = null
        videoFrame = null
        automatic = false
        hide()
    }

    // Close and show the tab in the browser window.
    function backToTab() {
        const tab = view
        restore(false)
        if (!tab)
            return
        const row = Tabs.indexOfTab(tab.tabId)
        if (row >= 0)
            Tabs.activate(row)
        browser.raise()
        browser.requestActivate()
    }

    function runInFrames(frame, script) {
        frame.runJavaScript(script, WebEngineScript.ApplicationWorld)
        for (const child of frame.children)
            runInFrames(child, script)
    }

    function control(action, seconds) {
        if (!videoFrame)
            return
        videoFrame.runJavaScript(Media.pipControlScript(action, seconds || 0),
                                 WebEngineScript.ApplicationWorld, result => {
            if (result)
                pip.videoState = JSON.parse(result)
        })
    }
    function refresh() { control("state") }

    function timeLabel(seconds) {
        const s = Math.max(0, Math.floor(seconds))
        const m = Math.floor(s / 60)
        const h = Math.floor(m / 60)
        const pad = n => (n < 10 ? "0" : "") + n
        return h > 0 ? h + ":" + pad(m % 60) + ":" + pad(s % 60) : m + ":" + pad(s % 60)
    }

    onWidthChanged: if (visible) preferredWidth = width

    // The pointer moved over the window lately: controls show, and hide
    // again once it rests, as in video players.
    property bool pointerActive: false
    HoverHandler {
        onPointChanged: {
            pip.pointerActive = true
            idleTimer.restart()
        }
        onHoveredChanged: if (!hovered) pip.pointerActive = false
    }
    Timer {
        id: idleTimer
        interval: 2500
        onTriggered: pip.pointerActive = false
    }

    Connections {
        target: pip.view
        // Leaving the page, or reloading it, ends picture-in-picture.
        function onUrlChanged() { pip.restore(false) }
        function onLoadingChanged(info) {
            if (info.status === WebEngineView.LoadStartedStatus)
                pip.restore(false)
        }
    }

    Timer {
        interval: 500
        repeat: true
        running: pip.visible
        onTriggered: pip.refresh()
    }

    // The tab's view goes in here.
    Item {
        id: content
        anchors.fill: parent
    }

    // Takes every click, so the page under it can't be clicked by accident:
    // a click plays or pauses, a drag moves the window.
    MouseArea {
        id: surface
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        property point pressedAt
        property bool moved: false
        onPressed: mouse => { pressedAt = Qt.point(mouse.x, mouse.y); moved = false }
        onPositionChanged: mouse => {
            if (pressed && !moved && Math.hypot(mouse.x - pressedAt.x, mouse.y - pressedAt.y) > 4) {
                moved = true
                pip.startSystemMove()
            }
        }
        onClicked: if (!moved) pip.control("toggle")
        onDoubleClicked: pip.backToTab()
    }

    // Edges resize the window.
    component Edge: MouseArea {
        required property int edges
        hoverEnabled: true
        onPressed: pip.startSystemResize(edges)
    }
    Edge { edges: Qt.LeftEdge; anchors { left: parent.left; top: parent.top; bottom: parent.bottom } width: 6; cursorShape: Qt.SizeHorCursor }
    Edge { edges: Qt.RightEdge; anchors { right: parent.right; top: parent.top; bottom: parent.bottom } width: 6; cursorShape: Qt.SizeHorCursor }
    Edge { edges: Qt.TopEdge; anchors { left: parent.left; right: parent.right; top: parent.top } height: 6; cursorShape: Qt.SizeVerCursor }
    Edge { edges: Qt.BottomEdge; anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 6; cursorShape: Qt.SizeVerCursor }
    Edge { edges: Qt.LeftEdge | Qt.TopEdge; anchors { left: parent.left; top: parent.top } width: 12; height: 12; cursorShape: Qt.SizeFDiagCursor }
    Edge { edges: Qt.RightEdge | Qt.BottomEdge; anchors { right: parent.right; bottom: parent.bottom } width: 12; height: 12; cursorShape: Qt.SizeFDiagCursor }
    Edge { edges: Qt.RightEdge | Qt.TopEdge; anchors { right: parent.right; top: parent.top } width: 12; height: 12; cursorShape: Qt.SizeBDiagCursor }
    Edge { edges: Qt.LeftEdge | Qt.BottomEdge; anchors { left: parent.left; bottom: parent.bottom } width: 12; height: 12; cursorShape: Qt.SizeBDiagCursor }

    // Controls, over the video while the pointer is on the window.
    Item {
        id: controls
        anchors.fill: parent
        opacity: pip.pointerActive || pip.videoState.paused ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: Theme.animationMs } }

        // Darkens the edges so white controls read on any video.
        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop { position: 0; color: Theme.videoScrim }
                GradientStop { position: 0.3; color: "transparent" }
                GradientStop { position: 0.7; color: "transparent" }
                GradientStop { position: 1; color: Theme.videoScrim }
            }
        }

        component PipButton: ToolButton {
            id: button
            property string iconName
            property string tip
            property int size: Theme.urlBarHeight
            implicitWidth: size
            implicitHeight: size
            focusPolicy: Qt.NoFocus
            contentItem: Item {
                Icon {
                    anchors.centerIn: parent
                    name: button.iconName
                    size: button.size * 0.55
                    color: Theme.onVideo
                }
            }
            background: Rectangle {
                radius: width / 2
                color: Theme.alpha(Theme.onVideo, button.down ? 0.3 : button.hovered ? 0.18 : 0)
            }
            ToolTip.visible: hovered && tip.length > 0
            ToolTip.text: tip
            ToolTip.delay: 600
        }

        PipButton {
            anchors { left: parent.left; top: parent.top; margins: Theme.spacing }
            iconName: "pip"
            tip: qsTr("Back to tab")
            onClicked: pip.backToTab()
        }
        PipButton {
            anchors { right: parent.right; top: parent.top; margins: Theme.spacing }
            iconName: "close"
            tip: qsTr("Close")
            onClicked: pip.restore(true)
        }

        Row {
            anchors.centerIn: parent
            spacing: Theme.spacing * 3
            PipButton {
                anchors.verticalCenter: parent.verticalCenter
                iconName: "restore"
                tip: qsTr("Back 10 seconds")
                onClicked: pip.control("seek", -10)
            }
            PipButton {
                size: Theme.urlBarHeight * 1.6
                iconName: pip.videoState.paused ? "play" : "pause"
                tip: pip.videoState.paused ? qsTr("Play") : qsTr("Pause")
                onClicked: pip.control("toggle")
            }
            PipButton {
                anchors.verticalCenter: parent.verticalCenter
                iconName: "reload"
                tip: qsTr("Forward 10 seconds")
                onClicked: pip.control("seek", 10)
            }
        }

        RowLayout {
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: Theme.spacing }
            spacing: Theme.spacing

            Text {
                visible: pip.videoState.duration > 0
                text: pip.timeLabel(pip.videoState.time) + " / " + pip.timeLabel(pip.videoState.duration)
                color: Theme.onVideo
                font.pixelSize: Theme.fontSizeSmall
                font.features: { "tnum": 1 }
            }
            // How far along the video is; a click jumps there.
            Rectangle {
                Layout.fillWidth: true
                height: Theme.hairline * 4
                radius: height / 2
                color: Theme.alpha(Theme.onVideo, 0.3)
                visible: pip.videoState.duration > 0
                Rectangle {
                    width: parent.width * Math.min(1, pip.videoState.time / Math.max(1, pip.videoState.duration))
                    height: parent.height
                    radius: parent.radius
                    color: Theme.accent
                }
                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -Theme.spacing
                    onClicked: mouse => {
                        const target = (mouse.x + anchors.margins) / parent.width * pip.videoState.duration
                        pip.control("seek", target - pip.videoState.time)
                    }
                }
            }
            Item { Layout.fillWidth: true; visible: pip.videoState.duration <= 0 }
            PipButton {
                iconName: pip.videoState.muted ? "mute" : "volume"
                tip: pip.videoState.muted ? qsTr("Unmute") : qsTr("Mute")
                onClicked: pip.control("mute")
            }
        }
    }

    Shortcut {
        sequence: "Space"
        context: Qt.WindowShortcut
        onActivated: pip.control("toggle")
    }
    Shortcut {
        sequence: "Escape"
        context: Qt.WindowShortcut
        onActivated: pip.backToTab()
    }
    Shortcut { sequence: "Left"; context: Qt.WindowShortcut; onActivated: pip.control("seek", -5) }
    Shortcut { sequence: "Right"; context: Qt.WindowShortcut; onActivated: pip.control("seek", 5) }
    Shortcut { sequence: "M"; context: Qt.WindowShortcut; onActivated: pip.control("mute") }

    // Closing the window from the window manager.
    onClosing: event => { event.accepted = false; pip.restore(true) }
}
