import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Back / forward / reload and the URL bar for the current tab.
Rectangle {
    id: bar

    property var view   // BrowserTab of the current tab, may be null
    property alias urlBar: urlBar

    implicitHeight: Theme.urlBarHeight + Theme.spacing * 2
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

        AdblockButton { view: bar.view }
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
