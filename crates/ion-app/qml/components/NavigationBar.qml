import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Back / forward / reload and the URL bar for the current tab.
Rectangle {
    id: bar

    property var view   // BrowserTab of the current tab, may be null
    property alias urlBar: urlBar

    signal bookmarkToggled()

    implicitHeight: Theme.urlBarHeight + Theme.spacing * 2

    // Switching tabs replaces any half-typed text with the new tab's address.
    // Deferred so it runs after urlBar.currentUrl has re-evaluated for the new view.
    onViewChanged: Qt.callLater(urlBar.showUrl)

    color: Theme.surface

    RowLayout {
        anchors.fill: parent
        anchors.margins: Theme.spacing
        spacing: Theme.spacing / 2

        IconButton {
            glyph: "←"
            tip: qsTr("Back")
            enabled: bar.view?.canGoBack ?? false
            onClicked: bar.view.goBack()
        }
        IconButton {
            glyph: "→"
            tip: qsTr("Forward")
            enabled: bar.view?.canGoForward ?? false
            onClicked: bar.view.goForward()
        }
        IconButton {
            readonly property bool loading: bar.view?.loading ?? false
            glyph: loading ? "✕" : "↻"
            tip: loading ? qsTr("Stop") : qsTr("Reload")
            enabled: bar.view !== null
            onClicked: loading ? bar.view.stop() : bar.view.reload()
        }

        UrlBar {
            id: urlBar
            Layout.fillWidth: true
            currentUrl: bar.view?.url ?? ""
            onNavigate: target => {
                if (bar.view)
                    bar.view.url = target
            }
            onFinished: {
                if (bar.view)
                    bar.view.forceActiveFocus()
                else
                    urlBar.focus = false
            }
        }
        IconButton {
            readonly property bool reading: bar.view?.reader.active ?? false
            glyph: "¶"
            tip: reading ? qsTr("Leave reader mode") : qsTr("Reader mode")
            enabled: bar.view !== null
            opacity: reading ? 1 : 0.7
            onClicked: bar.view.reader.toggle()
        }
        IconButton {
            readonly property string pageUrl: (bar.view?.url ?? "").toString()
            // Reading revision makes the binding re-run when bookmarks change.
            readonly property bool bookmarked: Bookmarks.revision >= 0 && Bookmarks.contains(pageUrl)
            glyph: bookmarked ? "★" : "☆"
            tip: bookmarked ? qsTr("Remove bookmark") : qsTr("Bookmark this page")
            enabled: pageUrl.startsWith("http") || pageUrl.startsWith("file:")
            onClicked: bar.bookmarkToggled()
        }

        AdblockButton { view: bar.view }
    }

    // Hairline between the browser chrome and the page.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.hairline
        color: Theme.border
    }

    // Load progress along the bottom edge. It eases forward instead of
    // jumping, runs to the end when the load finishes and then fades out.
    Rectangle {
        id: progressLine

        readonly property bool loading: bar.view?.loading ?? false
        // Never sits at 0 while loading, so a slow first byte still shows activity.
        readonly property real target: loading ? Math.max(0.08, (bar.view?.loadProgress ?? 0) / 100) : 1
        property real shown: 0

        anchors.left: parent.left
        anchors.bottom: parent.bottom
        height: 2
        width: parent.width * shown
        color: Theme.accent
        opacity: 0

        onLoadingChanged: {
            if (loading) {
                fade.stop()
                opacity = 1
            } else {
                fade.restart()
            }
        }

        // A new load (or a tab switch) can move the target backwards: restart
        // from the left edge rather than sliding back.
        onTargetChanged: {
            if (target < shown) {
                grow.stop()
                shown = 0
            }
            grow.to = target
            grow.restart()
        }

        NumberAnimation {
            id: grow
            target: progressLine
            property: "shown"
            duration: Theme.animationSlowMs
            easing.type: Easing.OutCubic
        }

        SequentialAnimation {
            id: fade
            PauseAnimation { duration: Theme.animationSlowMs }
            NumberAnimation { target: progressLine; property: "opacity"; to: 0; duration: Theme.animationSlowMs }
        }
    }
}
