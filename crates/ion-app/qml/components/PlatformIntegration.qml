import QtQuick
import Ion

// Desktop integration for the main window: URLs handed over by a second
// launch or by macOS (clicked links, files opened with Ion). macOS's own
// shortcuts are the commands' defaults there (`ion_bangs::commands`).
Item {
    id: root

    // The ApplicationWindow from Main.qml (openTab).
    required property var window

    Omnibox {
        id: resolver
        searchEngineName: Config.searchEngineName
        searchTemplate: Config.searchTemplate
    }

    Connections {
        target: Platform

        function onOpenRequested(urls, activationToken) {
            urls.forEach((url, i) => root.window.openTab(resolver.resolve(url), i === 0))
            if (root.window.visibility === Window.Minimized)
                root.window.showNormal()
            Platform.prepareActivation(activationToken)
            root.window.raise()
            root.window.requestActivate()
        }
    }
}
