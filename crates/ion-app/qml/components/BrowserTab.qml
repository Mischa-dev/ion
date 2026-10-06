import QtQuick
import QtWebEngine
import Ion

// One tab's web content. Thin wrapper over WebEngineView so per-tab features
// (adblock counters, agent indicators, per-site settings) have a home.
WebEngineView {
    id: view

    // Emitted when the page asks for a new tab or window (target=_blank, window.open).
    signal newTabRequested(var request)
    // Emitted when Ion's own UI (context menu) wants `target` opened in a new tab.
    signal openInNewTab(url target)

    // False for tabs a page opened (window.open), whose blank document the
    // page fills in itself; those must not be covered by the new-tab page.
    property bool showNewTabPage: true

    onNewWindowRequested: request => view.newTabRequested(request)

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

    onLoadingChanged: request => {
        if (request.status === WebEngineView.LoadSucceededStatus)
            zoomFactor = Zoom.factorFor(url)
    }

    NewTabPage { view: view }
    FindBar { view: view }
    PermissionPrompt { view: view }
    ScreenSharePicker { view: view }
    ContextMenuHandler {
        view: view
        omnibox: tabOmnibox
        onOpenInNewTab: target => view.openInNewTab(target)
    }
    Omnibox { id: tabOmnibox }
}
