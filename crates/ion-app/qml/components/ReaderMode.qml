import QtQuick
import QtWebEngine
import Ion

// Reader mode for one tab: swaps the page for a clean, themed copy of its
// article (`Reader.render`) and back. `view` is the tab's BrowserTab.
QtObject {
    id: root

    required property var view
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
                root.view.notify(qsTr("No article found on this page"))
                return
            }
            root.source = pageUrl
            root.view.loadHtml(page, pageUrl)
        })
    }

    // Following a link out of the reader page ends reader mode.
    property Connections urlWatch: Connections {
        target: root.view
        function onUrlChanged() {
            if (root.active && root.view.url.toString() !== root.source)
                root.source = ""
        }
    }
}
