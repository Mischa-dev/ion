import QtQuick
import QtWebEngine
import Ion

// Reader mode for one tab: swaps the page for a clean, themed copy of its
// article (`Reader.render`) and back. Lives over the web view so it can show
// a short notice when a page has no article.
Item {
    id: root

    required property WebEngineView view
    // The page reader mode was opened from; empty when it is off.
    property string source: ""
    readonly property bool active: source.length > 0

    function style() {
        return JSON.stringify({
            background: Theme.background.toString(),
            text: Theme.text.toString(),
            muted: Theme.textMuted.toString(),
            accent: Theme.accent.toString(),
            surface: Theme.surfaceRaised.toString(),
            border: Theme.border.toString()
        })
    }

    function toggle() {
        if (active) {
            const back = source
            source = ""
            view.url = back
            return
        }
        const pageUrl = view.url.toString()
        view.runJavaScript("document.documentElement.outerHTML", WebEngineScript.ApplicationWorld, html => {
            const page = html ? Reader.render(html, pageUrl, root.style()) : ""
            if (page.length === 0) {
                notice.show()
                return
            }
            root.source = pageUrl
            root.view.loadHtml(page, pageUrl)
        })
    }

    // Following a link out of the reader page ends reader mode.
    Connections {
        target: root.view
        function onUrlChanged() {
            if (root.active && root.view.url.toString() !== root.source)
                root.source = ""
        }
    }

    anchors.fill: parent
    // Above the page content, which the view adds as a child of its own.
    z: 1

    Rectangle {
        id: notice

        function show() {
            opacity = 1
            hideTimer.restart()
        }

        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        anchors.topMargin: Theme.spacing * 3
        width: label.implicitWidth + Theme.spacing * 4
        height: label.implicitHeight + Theme.spacing * 2
        radius: Theme.radius
        color: Theme.surfaceRaised
        border.width: 1
        border.color: Theme.border
        opacity: 0
        visible: opacity > 0

        Behavior on opacity { NumberAnimation { duration: Theme.animationMs } }

        Text {
            id: label
            anchors.centerIn: parent
            text: qsTr("No article found on this page")
            color: Theme.text
            font.pixelSize: Theme.fontSize
        }

        Timer {
            id: hideTimer
            interval: Theme.noticeMs
            onTriggered: notice.opacity = 0
        }
    }
}
