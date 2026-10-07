import QtQuick
import QtWebEngine
import Ion

// One tab's web content. Thin wrapper over WebEngineView so per-tab features
// (adblock counters, agent indicators, per-site settings) have a home.
WebEngineView {
    id: view

    // Emitted when the page asks for a new tab or window (target=_blank, window.open).
    signal newTabRequested(var request)

    // Pages follow Ion's theme (theme.pages). QtWebEngine reads the light/dark
    // preference and force-dark only when a page's settings are applied, so
    // setting forceDarkMode (even to the same value) re-applies them.
    function applyPageTheme() {
        settings.forceDarkMode = Theme.engine.darkenPages
    }

    Component.onCompleted: applyPageTheme()

    // Creating a tab resets the scheme QtWebEngine hands to pages, so restore
    // Ion's before each load.
    onLoadingChanged: info => {
        if (info.status === WebEngineView.LoadStartedStatus) {
            Theme.engine.applyPageScheme()
            applyPageTheme()
        }
    }
    onNewWindowRequested: request => view.newTabRequested(request)

    Connections {
        target: Theme.engine
        function onPageSchemeChanged() {
            view.applyPageTheme()
        }
    }
}
