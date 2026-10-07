import QtQuick
import QtWebEngine

// One tab's web content. Thin wrapper over WebEngineView so per-tab features
// (adblock counters, agent indicators, per-site settings) have a home.
WebEngineView {
    id: view

    // Emitted when the page asks for a new tab or window (target=_blank, window.open).
    signal newTabRequested(var request)

    onNewWindowRequested: request => view.newTabRequested(request)

    AdblockCosmetics { view: view }
}
