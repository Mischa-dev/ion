import QtQuick
import Ion

// Desktop integration for the main window: URLs handed over by a second
// launch or by macOS (clicked links, files opened with Ion), and the macOS
// shortcuts that Main.qml's cross-platform ones do not cover.
Item {
    id: root

    // The ApplicationWindow from Main.qml (openTab).
    required property var window

    readonly property bool macOS: Qt.platform.os === "osx"

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

    // Qt maps "Ctrl" to Cmd and "Meta" to the Control key on macOS, so the
    // shared Ctrl+Tab becomes Cmd+Tab there, which the system takes.
    Shortcut {
        enabled: root.macOS
        sequences: ["Meta+Tab", "Ctrl+}", "Ctrl+Alt+Right"]
        onActivated: Tabs.cycle(1)
    }
    Shortcut {
        enabled: root.macOS
        sequences: ["Meta+Shift+Tab", "Ctrl+{", "Ctrl+Alt+Left"]
        onActivated: Tabs.cycle(-1)
    }
}
