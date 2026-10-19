import QtQuick
import QtWebEngine
import Ion

// Ion's own pages (`ion://settings`, `ion://history`…), drawn over the tab.
// The `ion:` scheme serves the tab an empty, titled document (bridge/pages.rs),
// so Back, Reload and session restore treat these like any other page.
Loader {
    id: loader

    required property WebEngineView view

    readonly property string page: Pages.pageOf(view.url)
    readonly property var files: ({
        settings: "SettingsPage",
        history: "HistoryPage",
        downloads: "DownloadsPage",
        bookmarks: "BookmarksPage",
        welcome: "WelcomePage"
    })

    anchors.fill: parent
    // Above the new-tab page and the error page.
    z: 6
    active: files[page] !== undefined
    visible: active
    source: active ? "qrc:/qt/qml/Ion/qml/pages/" + files[page] + ".qml" : ""

    // Clicks and scrolling on the page's empty parts stop here instead of
    // reaching the blank web page underneath (and its context menu).
    MouseArea {
        anchors.fill: parent
        z: -1
        enabled: loader.active
        acceptedButtons: Qt.AllButtons
        onWheel: wheel => wheel.accepted = true
    }

    // Ctrl+F on one of these pages focuses its own search field.
    function focusSearch() {
        if (item && item.focusSearch)
            item.focusSearch()
    }

    onLoaded: {
        item.view = Qt.binding(() => loader.view)
        item.path = Qt.binding(() => Pages.pathOf(loader.view.url))
    }
}
