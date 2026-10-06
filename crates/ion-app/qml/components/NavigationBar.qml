import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Back / forward / reload and the URL bar for the current tab.
Rectangle {
    id: bar

    property var view   // BrowserTab of the current tab, may be null
    property DownloadsPanel downloads   // may be null
    property alias urlBar: urlBar

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
        }

        IconButton {
            readonly property real factor: bar.view?.zoomFactor ?? 1
            visible: bar.view !== null && !Zoom.isDefault(factor)
            implicitWidth: Theme.urlBarHeight * 1.6
            glyph: Zoom.label(factor)
            tip: qsTr("Reset zoom")
            onClicked: bar.view.resetZoom()
        }

        DownloadsButton { panel: bar.downloads }
    }

    // Thin load progress line along the bottom edge.
    Rectangle {
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        height: 2
        color: Theme.accent
        visible: bar.view?.loading ?? false
        width: parent.width * ((bar.view?.loadProgress ?? 0) / 100)
    }
}
