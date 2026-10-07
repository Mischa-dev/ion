import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

ApplicationWindow {
    id: window

    readonly property string homeUrl: Config.homePage
    readonly property bool verticalTabs: Config.tabLayout === "vertical"
    // Bumped whenever the view stack gains or loses a view, so `currentView`
    // re-evaluates even when the count and current index happen to stay the same.
    property int viewsRevision: 0
    readonly property var currentView: {
        window.viewsRevision
        const index = Tabs.currentIndex
        return window.viewAt(index)
    }

    width: 1280
    height: 820
    visible: true
    color: Theme.background
    title: currentView && currentView.title ? currentView.title + " — Ion" : "Ion"

    // Persistent profile: cookies, cache and storage survive restarts.
    WebEngineProfilePrototype {
        id: profilePrototype
        storageName: "Default"
    }
    // Set in Component.onCompleted: instance() is null until the prototype is complete.
    property WebEngineProfile profile: null

    // Tabs live in the Rust `Tabs` model (bridge/tabs.rs), which also saves them
    // so the next start reopens them. These helpers return the new tab's view.
    function openTab(target, activate) {
        return views.itemAt(Tabs.openTab(target ? target.toString() : "", activate !== false))
    }

    // Tabs a page opens (links with target=_blank, window.open) go next to it.
    function openPageTab(activate) {
        return views.itemAt(Tabs.openTabNextToCurrent("", activate))
    }

    function closeTab(index) {
        if (index < 0 || index >= Tabs.count)
            return
        // Closing the last tab closes the window; the tab stays in the saved
        // session so the next start brings it back.
        if (Tabs.count === 1) {
            Qt.quit()
            return
        }
        Tabs.closeTab(index)
    }

    function viewAt(index) {
        return index >= 0 && index < views.count ? views.itemAt(index) : null
    }

    // For the tab menu. Closing from the end keeps the lower indexes valid.
    function closeTabsAfter(index) {
        for (let i = Tabs.count - 1; i > index; i--)
            Tabs.closeTab(i)
    }

    function closeOtherTabs(index) {
        closeTabsAfter(index)
        for (let i = index - 1; i >= 0; i--)
            Tabs.closeTab(i)
    }

    function newTab() {
        window.openTab("")
        window.focusUrlBar()
    }

    // Keyboard focus follows the visible page (so Space, arrows and Find work
    // right after switching tabs), unless the person is typing in the URL bar.
    onCurrentViewChanged: Qt.callLater(focusPage)
    function focusPage() {
        if (currentView && !navBar.urlBar.activeFocus && !palette.opened)
            currentView.forceActiveFocus()
    }

    function focusUrlBar() {
        navBar.urlBar.forceActiveFocus()
        navBar.urlBar.selectAll()
    }

    Component.onCompleted: {
        profile = profilePrototype.instance()
        Adblock.attach(window.profile)
        if (Config.value("general.restoreSession"))
            Tabs.restoreLastSession()
        // URLs on the command line (`ion %U` from the desktop file) open as new
        // tabs after the restored ones; the first one becomes current.
        const urls = Qt.application.arguments.slice(1).filter(arg => !arg.startsWith("-"))
        urls.forEach((arg, i) => openTab(urlBarResolver.resolve(arg), i === 0))
        if (Tabs.count === 0)
            openTab(homeUrl)
    }

    // Save tabs shortly after they change, and history less eagerly; both are
    // written once more on quit.
    Timer {
        id: sessionSaveTimer
        interval: 1000
        onTriggered: Tabs.saveSession()
    }
    // Unload background tabs nobody has looked at for `general.suspendTabsAfter`
    // minutes. Tabs playing sound are left alone.
    Timer {
        interval: 60 * 1000
        repeat: true
        running: true
        onTriggered: {
            const minutes = Config.value("general.suspendTabsAfter")
            if (!(minutes > 0))
                return
            for (const index of Tabs.idleTabs(minutes * 60)) {
                const view = views.itemAt(index)
                if (view && !view.recentlyAudible)
                    Tabs.setSuspended(index, true)
            }
        }
    }
    Timer {
        id: historySaveTimer
        interval: 5000
        onTriggered: History.save()
    }
    Connections {
        target: Tabs
        function onSessionChanged() { sessionSaveTimer.restart() }
    }
    Connections {
        target: History
        function onChanged() { historySaveTimer.restart() }
    }
    Connections {
        target: Qt.application
        function onAboutToQuit() {
            Tabs.saveSession()
            History.save()
        }
    }

    Omnibox {
        id: urlBarResolver
        searchEngineName: Config.searchEngineName
        searchTemplate: Config.searchTemplate
    }

    PlatformIntegration { window: window }

    // Command palette (Ctrl/Cmd+K).
    CommandPalette { id: palette; browser: window }

    header: ColumnLayout {
        spacing: 0

        TabStrip {
            Layout.fillWidth: true
            visible: !window.verticalTabs
            tabs: Tabs
            currentIndex: Tabs.currentIndex
            onActivated: index => Tabs.activate(index)
            onCloseRequested: index => window.closeTab(index)
            onMoveRequested: (from, to) => Tabs.moveTab(from, to)
            onNewTabRequested: window.newTab()
            onMenuRequested: anchor => sessionMenu.open(anchor)
            onTabMenuRequested: index => tabMenu.openFor(index)
            views: views
            viewsRevision: window.viewsRevision
        }

        NavigationBar {
            id: navBar
            Layout.fillWidth: true
            view: window.currentView
        }
    }

    SessionMenu { id: sessionMenu }
    TabMenu { id: tabMenu; browser: window }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        VerticalTabStrip {
            Layout.fillHeight: true
            // Above the page, which an expanded collapsed sidebar covers.
            z: 1
            visible: window.verticalTabs
            collapsed: {
                Config.revision
                return Config.value("ui.collapseSidebar") === true
            }
            tabs: Tabs
            currentIndex: Tabs.currentIndex
            onActivated: index => Tabs.activate(index)
            onCloseRequested: index => window.closeTab(index)
            onMoveRequested: (from, to) => Tabs.moveTab(from, to)
            onNewTabRequested: window.newTab()
            onMenuRequested: anchor => sessionMenu.open(anchor)
            onTabMenuRequested: index => tabMenu.openFor(index)
            views: views
            viewsRevision: window.viewsRevision
        }

        // One web view per tab, stacked; only the current one is visible.
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            Repeater {
                id: views
                model: Tabs

                onItemAdded: window.viewsRevision++
                onItemRemoved: window.viewsRevision++

                delegate: BrowserTab {
                    id: view

                    required property int index
                    required property bool suspended
                    readonly property bool current: index === Tabs.currentIndex
                    // Background tabs restored from a session load when first shown.
                    property bool deferred: false

                    function loadSavedUrl() {
                        const target = Tabs.urlAt(index)
                        if (target.length > 0)
                            url = target
                    }

                    anchors.fill: parent
                    visible: current
                    profile: window.profile
                    // A suspended tab's page is unloaded; it reloads when shown.
                    lifecycleState: suspended ? WebEngineView.LifecycleState.Discarded
                                              : WebEngineView.LifecycleState.Active

                    Component.onCompleted: {
                        deferred = Tabs.restoring && !current
                        if (!deferred)
                            loadSavedUrl()
                    }
                    onCurrentChanged: {
                        if (current && deferred) {
                            deferred = false
                            loadSavedUrl()
                        }
                    }

                    onUrlChanged: {
                        if (!deferred)
                            Tabs.setUrl(index, url.toString())
                    }
                    onTitleChanged: {
                        if (deferred)
                            return
                        Tabs.setTitle(index, title)
                        History.updateTitle(url.toString(), title)
                    }
                    onIconChanged: {
                        if (!deferred)
                            Tabs.setIcon(index, icon.toString())
                    }
                    onLoadingChanged: info => {
                        if (info.status === WebEngineView.LoadSucceededStatus)
                            History.recordVisit(info.url.toString(), title)
                    }
                    onNewTabRequested: request => {
                        const background = request.destination === WebEngineNewWindowRequest.InNewBackgroundTab
                        request.openIn(window.openPageTab(!background))
                    }
                }
            }
        }
    }

    // Keyboard shortcuts. "Ctrl" maps to Cmd on macOS automatically.
    Shortcut { sequences: [StandardKey.AddTab]; onActivated: window.newTab() }
    Shortcut { sequences: [StandardKey.Close]; onActivated: window.closeTab(Tabs.currentIndex) }
    Shortcut { sequence: "Ctrl+Shift+T"; onActivated: Tabs.reopenClosedTab() }
    Shortcut { sequences: ["Ctrl+L", "Alt+D", "F6"]; onActivated: window.focusUrlBar() }
    Shortcut { sequences: ["Ctrl+Tab", "Ctrl+PgDown"]; onActivated: Tabs.cycle(1) }
    Shortcut { sequences: ["Ctrl+Shift+Tab", "Ctrl+PgUp"]; onActivated: Tabs.cycle(-1) }
    Shortcut { sequence: "Ctrl+Shift+PgDown"; onActivated: Tabs.moveTab(Tabs.currentIndex, Tabs.currentIndex + 1) }
    Shortcut { sequence: "Ctrl+Shift+PgUp"; onActivated: Tabs.moveTab(Tabs.currentIndex, Tabs.currentIndex - 1) }
    // Ctrl+1…8 pick a tab by position, Ctrl+9 the last one.
    Shortcut { sequence: "Ctrl+1"; onActivated: Tabs.activate(0) }
    Shortcut { sequence: "Ctrl+2"; onActivated: Tabs.activate(1) }
    Shortcut { sequence: "Ctrl+3"; onActivated: Tabs.activate(2) }
    Shortcut { sequence: "Ctrl+4"; onActivated: Tabs.activate(3) }
    Shortcut { sequence: "Ctrl+5"; onActivated: Tabs.activate(4) }
    Shortcut { sequence: "Ctrl+6"; onActivated: Tabs.activate(5) }
    Shortcut { sequence: "Ctrl+7"; onActivated: Tabs.activate(6) }
    Shortcut { sequence: "Ctrl+8"; onActivated: Tabs.activate(7) }
    Shortcut { sequence: "Ctrl+9"; onActivated: Tabs.activate(Tabs.count - 1) }
    Shortcut { sequences: [StandardKey.Refresh, "Ctrl+R"]; onActivated: window.currentView?.reload() }
    Shortcut { sequences: [StandardKey.Back]; onActivated: window.currentView?.goBack() }
    Shortcut { sequences: [StandardKey.Forward]; onActivated: window.currentView?.goForward() }
    Shortcut { sequences: [StandardKey.Quit]; onActivated: Qt.quit() }
    Shortcut {
        sequence: { Config.revision; return Config.value("shortcuts.palette") || "Ctrl+K" }
        onActivated: palette.show()
    }
    // Stops every agent at once (docs/SAFETY.md). Resuming is deliberate:
    // the notice's button or the palette, never the same key again.
    Shortcut {
        sequence: { Config.revision; return Config.value("shortcuts.stopAgents") || "Ctrl+Shift+Escape" }
        context: Qt.ApplicationShortcut
        onActivated: Safety.stopAgents()
    }

    AgentsStoppedNotice {}
}
