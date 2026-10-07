import QtQuick
import QtWebEngine
import Ion

// One tab's web content. Thin wrapper over WebEngineView so per-tab features
// (adblock counters, agent indicators, per-site settings) have a home.
WebEngineView {
    id: view

    // Reader mode for this tab: `reader.toggle()`, `reader.active`.
    readonly property alias reader: readerMode

    // Emitted when the page asks for a new tab or window (target=_blank, window.open).
    signal newTabRequested(var request)

    // A tab with no page yet matches the browser chrome instead of glaring
    // white. Pages get the web's usual white canvas, which many rely on.
    backgroundColor: url.toString().length === 0 || url.toString() === "about:blank"
        ? Theme.background : Theme.pageCanvas

    // Pages follow Ion's theme (theme.pages, [theme.sites]). QtWebEngine reads
    // the light/dark preference and force-dark only when a page's settings
    // are applied, so setting forceDarkMode (even to the same value)
    // re-applies them.
    function applyPageTheme() {
        settings.forceDarkMode = Theme.engine.darkenPage(url.toString())
        applySiteCss()
    }

    // Page and per-site CSS come from one user script that matches hosts
    // itself. It runs on every new document, frames included; running it in
    // each frame now updates the open page.
    property string siteCssScript: ""
    function applySiteCss() {
        const script = Theme.engine.siteCssScript
        if (script === siteCssScript)
            return
        const old = userScripts.find("ion-site-css")
        if (old.length > 0)
            userScripts.remove(old[0])
        if (script.length > 0) {
            const userScript = WebEngine.script()
            userScript.name = "ion-site-css"
            userScript.sourceCode = script
            userScript.injectionPoint = WebEngineScript.DocumentCreation
            userScript.worldId = WebEngineScript.ApplicationWorld
            userScript.runsOnSubFrames = true
            userScripts.insert(userScript)
            runInFrames(mainFrame, script)
        } else {
            runInFrames(mainFrame, Theme.engine.clearSiteCssScript())
        }
        siteCssScript = script
    }
    function runInFrames(frame, script) {
        frame.runJavaScript(script, WebEngineScript.ApplicationWorld)
        for (const child of frame.children)
            runInFrames(child, script)
    }

    Component.onCompleted: applyPageTheme()

    // A site of its own may darken differently.
    onUrlChanged: {
        const darken = Theme.engine.darkenPage(url.toString())
        if (settings.forceDarkMode !== darken)
            settings.forceDarkMode = darken
    }

    // Creating a tab resets the scheme QtWebEngine hands to pages, so restore
    // Ion's when this tab's first load starts. Later loads don't reset it.
    property bool pageSchemeRestored: false
    onLoadingChanged: info => {
        if (!pageSchemeRestored && info.status === WebEngineView.LoadStartedStatus) {
            pageSchemeRestored = true
            Theme.engine.applyPageScheme()
            applyPageTheme()
        }
    }
    onNewWindowRequested: request => view.newTabRequested(request)

    // Show a short message over the page.
    function notify(message) {
        notice.show(message)
    }

    ReaderMode { id: readerMode; view: view }
    PageNotice { id: notice }

    // Per-site JavaScript switch from `[sites]` in config, applied as each
    // page starts loading.
    onNavigationRequested: request => {
        if (request.isMainFrame)
            view.settings.javascriptEnabled = Sites.javascriptEnabled(request.url.toString())
    }

    Connections {
        target: Theme.engine
        function onPageSchemeChanged() {
            view.applyPageTheme()
        }
    }
}
