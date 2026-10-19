import QtQuick
import Ion

// Carries out Ion's commands (`ion_bangs::commands::COMMANDS`), whether they
// come from a shortcut, the palette or anywhere else, and binds each one's
// keys from `Keymap` (defaults plus `[shortcuts]`).
//
// `browser` (the main window) provides openTab(url), newTab(), closeTab(i),
// closeOtherTabs(i), focusUrlBar(), toggleBookmark(), showImport(),
// showExtensions(), toggleFullScreen(), askAgent(input), startAgentQuestion(),
// toggleAgentSidebar(), showAgentActivity(), openPage(id, path),
// showClearData(), profile and currentView.
Item {
    id: root

    required property var browser
    // The command palette (show(text)), the shortcut editor (show()) and the
    // downloads panel (toggle()).
    required property var commandPalette
    required property var shortcutEditor
    required property var downloads

    readonly property var agentCommands: ["ask-agent", "summarize-page", "agent-sidebar", "agent-activity"]

    // Whether `id` can run right now: agent commands need agents turned on.
    function available(id) {
        Config.revision
        return !agentCommands.includes(id) || Config.value("ai.enable") === true
    }

    function copyText(text) {
        clipboard.text = text
        clipboard.selectAll()
        clipboard.copy()
    }

    function run(id) {
        if (!available(id))
            return
        const view = browser.currentView
        switch (id) {
        case "new-tab": browser.newTab(); break
        case "close-tab": browser.closeTab(Tabs.currentIndex); break
        case "duplicate-tab": browser.openTab(view ? view.url : ""); break
        case "reopen-tab": Tabs.reopenClosedTab(); break
        case "next-tab": Tabs.cycle(1); break
        case "previous-tab": Tabs.cycle(-1); break
        case "move-tab-left": Tabs.moveTab(Tabs.currentIndex, Tabs.neighbour(Tabs.currentIndex, -1)); break
        case "move-tab-right": Tabs.moveTab(Tabs.currentIndex, Tabs.neighbour(Tabs.currentIndex, 1)); break
        case "next-workspace": Tabs.cycleWorkspace(1); break
        case "previous-workspace": Tabs.cycleWorkspace(-1); break
        case "close-other-tabs": browser.closeOtherTabs(Tabs.currentIndex); break
        case "focus-url": browser.focusUrlBar(); break
        case "copy-url": copyText(view ? view.url.toString() : ""); break
        case "reload": view?.reload(); break
        case "hard-reload": view?.reloadAndBypassCache(); break
        case "stop": view?.stop(); break
        case "find":
            if (view?.builtinPage.active)
                view.builtinPage.focusSearch()
            else
                view?.findBar.open()
            break
        case "find-next": view?.findBar.step(1); break
        case "find-previous": view?.findBar.step(-1); break
        case "zoom-in": view?.zoomIn(); break
        case "zoom-out": view?.zoomOut(); break
        case "zoom-reset": view?.resetZoom(); break
        case "back": view?.goBack(); break
        case "forward": view?.goForward(); break
        case "bookmark-page": browser.toggleBookmark(); break
        case "bookmarks": Qt.callLater(() => commandPalette.show("*")); break
        case "import-browser-data": browser.showImport(); break
        case "clear-cookies": Privacy.clearCookies(browser.profile); break
        case "clear-cache": browser.profile.clearHttpCache(); break
        case "clear-history": History.clear(); History.save(); break
        case "reader-mode": view?.reader.toggle(); break
        case "extensions": browser.showExtensions(); break
        case "open-extensions-folder":
            if (Extensions.ensureFolder())
                Qt.openUrlExternally("file://" + Extensions.folder)
            break
        case "reload-userscripts": Sites.reload(); view?.reload(); break
        case "open-userscripts":
            if (Sites.ensureDirectory())
                Qt.openUrlExternally("file://" + Sites.directory)
            break
        case "fullscreen": browser.toggleFullScreen(); break
        case "toggle-adblock":
            if (view && Adblock.setEnabledOn(view.url, !Adblock.isEnabledOn(view.url)))
                view.reload()
            break
        case "update-filter-lists": Adblock.updateLists(); break
        case "stop-agents": Safety.stopAgents(); break
        case "resume-agents": Safety.resumeAgents(); break
        case "ask-agent": browser.startAgentQuestion(); break
        case "summarize-page": browser.askAgent("@ion Summarize this page."); break
        case "agent-sidebar": browser.toggleAgentSidebar(); break
        case "agent-activity": browser.showAgentActivity(); break
        case "screenshot": view?.takeScreenshot(false); break
        case "screenshot-page": view?.takeScreenshot(true); break
        case "downloads": downloads.toggle(); break
        case "keyboard-shortcuts": Qt.callLater(() => shortcutEditor.show()); break
        case "palette": commandPalette.show(); break
        case "tab-1": Tabs.activateNth(0); break
        case "tab-2": Tabs.activateNth(1); break
        case "tab-3": Tabs.activateNth(2); break
        case "tab-4": Tabs.activateNth(3); break
        case "tab-5": Tabs.activateNth(4); break
        case "tab-6": Tabs.activateNth(5); break
        case "tab-7": Tabs.activateNth(6); break
        case "tab-8": Tabs.activateNth(7); break
        case "last-tab": Tabs.activateNth(-1); break
        case "settings": browser.openPage("settings"); break
        case "clear-browsing-data": browser.showClearData(); break
        case "quit": Qt.quit(); break
        default: console.warn("Commands: unknown command", id)
        }
    }

    // QML has no clipboard API; an invisible text field copies for copyText().
    TextInput { id: clipboard; visible: false }

    // One Shortcut per command with keys. Rebuilt when the config changes.
    Repeater {
        model: {
            Config.revision
            return Keymap.bindings().filter(binding => binding.keys.length > 0)
        }

        delegate: Item {
            required property var modelData

            Shortcut {
                sequences: modelData.keys
                enabled: root.available(modelData.id)
                // Stopping agents works from anywhere, dialogs included.
                context: modelData.id === "stop-agents" ? Qt.ApplicationShortcut : Qt.WindowShortcut
                onActivated: root.run(modelData.id)
            }
        }
    }
}
