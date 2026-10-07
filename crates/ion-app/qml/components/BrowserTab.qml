import QtQuick
import QtWebEngine
import Ion

// One tab's web content. Thin wrapper over WebEngineView so per-tab features
// (adblock counters, agent indicators, per-site settings) have a home.
WebEngineView {
    id: view

    // Emitted when the page asks for a new tab or window (target=_blank, window.open).
    signal newTabRequested(var request)

    onNewWindowRequested: request => view.newTabRequested(request)

    // Per-site JavaScript switch from `[sites]` in config, applied as each
    // page starts loading.
    onNavigationRequested: request => {
        if (request.isMainFrame)
            view.settings.javascriptEnabled = Sites.javascriptEnabled(request.url.toString())
    }
}
