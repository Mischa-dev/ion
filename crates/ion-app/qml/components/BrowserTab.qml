import QtQuick
import QtWebEngine
import Ion

// One tab's web content. Thin wrapper over WebEngineView so per-tab features
// (adblock counters, agent indicators, per-site settings) have a home.
WebEngineView {
    id: view

    // The tab's id in the `Tabs` model, stable while the tab is open.
    required property int tabId

    // Reader mode for this tab: `reader.toggle()`, `reader.active`.
    readonly property alias reader: readerMode

    // Emitted when the page asks for a new tab or window (target=_blank, window.open).
    signal newTabRequested(var request)
    // Emitted when Ion's own UI (context menu) wants `target` opened in a new tab.
    signal openInNewTab(url target)

    // False for tabs a page opened (window.open), whose blank document the
    // page fills in itself; those must not be covered by the new-tab page.
    property bool showNewTabPage: true

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
        // Ion's own error page covers Chromium's.
        if (info.status === WebEngineView.LoadFailedStatus)
            loadError.showFor(info)
        else if (info.status === WebEngineView.LoadStartedStatus)
            loadError.visible = false
        if (!pageSchemeRestored && info.status === WebEngineView.LoadStartedStatus) {
            pageSchemeRestored = true
            Theme.engine.applyPageScheme()
            applyPageTheme()
        }
        // Failed loads too, so an error page doesn't keep the last site's zoom.
        if (info.status === WebEngineView.LoadSucceededStatus
                || info.status === WebEngineView.LoadFailedStatus)
            zoomFactor = Zoom.factorFor(url)
    }
    onNewWindowRequested: request => view.newTabRequested(request)
    // Keyboard focus belongs to the error page while it covers the view.
    onActiveFocusChanged: if (activeFocus && loadError.visible) loadError.takeFocus()

    // Both are off by default; Ion asks before a page uses them.
    settings.fullScreenSupportEnabled: true
    settings.screenCaptureEnabled: true

    // Zoom, remembered per site by the `Zoom` singleton.
    function setZoom(factor) {
        zoomFactor = factor
        Zoom.remember(url, factor)
    }
    function zoomIn() { setZoom(Zoom.stepIn(zoomFactor)) }
    function zoomOut() { setZoom(Zoom.stepOut(zoomFactor)) }
    function resetZoom() { setZoom(1.0) }

    // Saves the visible area (or the whole page) as a PNG in the downloads
    // folder and copies it to the clipboard.
    function takeScreenshot(fullPage) { screenshotTool.capture(fullPage) }

    NewTabPage { view: view }
    AdblockCosmetics { view: view }
    FindBar { view: view }
    PermissionPrompt { view: view; tabId: view.tabId }
    ScreenSharePicker { view: view }
    PageDialog { view: view }
    ScreenshotTool {
        id: screenshotTool
        view: view
    }
    ContextMenuHandler {
        view: view
        omnibox: tabOmnibox
        onOpenInNewTab: target => view.openInNewTab(target)
    }
    Omnibox {
        id: tabOmnibox
        searchEngineName: Config.searchEngineName
        searchTemplate: Config.searchTemplate
    }
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

    // Failed loads show LoadErrorPage, in Ion's colors, over Chromium's error
    // page. That page stays enabled underneath because it makes the failed
    // address the current entry, so the URL bar, title, Reload and Back treat
    // it like any other page.
    LoadErrorPage {
        id: loadError
        anchors.fill: parent
        onRetry: view.reload()
    }
}
