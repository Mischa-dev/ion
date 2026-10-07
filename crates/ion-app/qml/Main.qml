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
        return index >= 0 && index < views.count ? views.itemAt(index) : null
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

    function newTab() {
        window.openTab("")
        window.focusUrlBar()
    }

    // Bookmark the current page, or remove its bookmark (Ctrl+D, the URL bar star).
    function toggleBookmark() {
        const view = window.currentView
        if (view)
            Bookmarks.toggle(view.url.toString(), view.title)
    }

    function showImport() {
        importDialog.show()
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

    ImportDialog { id: importDialog }

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
        }

        NavigationBar {
            id: navBar
            Layout.fillWidth: true
            view: window.currentView
            onBookmarkToggled: window.toggleBookmark()
        }
    }

    SessionMenu { id: sessionMenu }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        VerticalTabStrip {
            Layout.fillHeight: true
            visible: window.verticalTabs
            tabs: Tabs
            currentIndex: Tabs.currentIndex
            onActivated: index => Tabs.activate(index)
            onCloseRequested: index => window.closeTab(index)
            onMoveRequested: (from, to) => Tabs.moveTab(from, to)
            onNewTabRequested: window.newTab()
            onMenuRequested: anchor => sessionMenu.open(anchor)
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
    Shortcut { sequence: "Ctrl+D"; onActivated: window.toggleBookmark() }
    Shortcut {
        sequence: { Config.revision; return Config.value("shortcuts.palette") || "Ctrl+K" }
        onActivated: palette.show()
    }
}
