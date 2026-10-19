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
    // The style's own controls (tooltips, scroll bars, text cursors) in the
    // theme's colors rather than the platform's.
    palette {
        window: Theme.background
        windowText: Theme.text
        base: Theme.surfaceRaised
        text: Theme.text
        button: Theme.surfaceRaised
        buttonText: Theme.text
        highlight: Theme.accent
        highlightedText: Theme.onAccent
        toolTipBase: Theme.surfaceRaised
        toolTipText: Theme.text
        mid: Theme.surfaceHover
        dark: Theme.border
        light: Theme.surfaceHover
        placeholderText: Theme.textMuted
    }
    // A private window or another profile says so in its title.
    readonly property bool privateWindow: Privacy.windowKind() === "private"
    readonly property string appTitle: Privacy.profileName().length > 0 ? "Ion — " + Privacy.profileName() : "Ion"
    title: currentView && currentView.title ? currentView.title + " — " + appTitle : appTitle

    // Persistent profile: cookies, cache and storage survive restarts.
    WebEngineProfilePrototype {
        id: profilePrototype
        // "Default", a profile's own, or "" (off the record) in a private window.
        storageName: Privacy.storageName()
        // Site permissions are decided and remembered by Ion's safety model
        // (Safety, docs/SAFETY.md), so the engine keeps none of its own.
        persistentPermissionsPolicy: WebEngineProfile.PersistentPermissionsPolicy.AskEveryTime
    }
    // The downloads panel's requests, newest first (the Downloads page lists
    // the running ones).
    readonly property var downloadItems: downloads.items
    // Set in Component.onCompleted: instance() is null until the prototype is complete.
    property WebEngineProfile profile: null
    // Folders of the Chrome extensions Ion loaded at startup.
    property var extensionPaths: []

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
        // A workspace's last tab makes way for a blank one, so closing it
        // doesn't jump to another workspace.
        if (index === Tabs.currentIndex && Tabs.pinnedAt(index) !== 2
                && Tabs.workspaceTabCount(Tabs.workspace) === 1)
            Tabs.openTabAt(index + 1, "", true)
        Tabs.closeTab(index)
    }

    function viewAt(index) {
        return index >= 0 && index < views.count ? views.itemAt(index) : null
    }

    // For the tab menu, within the tab's strip. Pinned tabs and Essentials
    // stay. Closing from the end keeps the lower indexes valid.
    function closeTabsAfter(index) {
        const rows = []
        for (let i = Tabs.neighbour(index, 1); i >= 0; i = Tabs.neighbour(i, 1)) {
            if (Tabs.pinnedAt(i) === 0)
                rows.push(i)
        }
        rows.reverse().forEach(i => Tabs.closeTab(i))
    }

    function closeOtherTabs(index) {
        closeTabsAfter(index)
        const rows = []
        for (let i = Tabs.neighbour(index, -1); i >= 0; i = Tabs.neighbour(i, -1)) {
            if (Tabs.pinnedAt(i) === 0)
                rows.push(i)
        }
        rows.forEach(i => Tabs.closeTab(i))
    }

    // The tab used before the current one in this workspace (or an
    // Essential), or -1. Never one from another workspace: showing it would
    // switch workspace, and mid-drag that drops the dragged tab's row.
    function previousTab() {
        return Tabs.previousTab()
    }

    // A tab dragged off a strip (`index` -1 once it is back or let go). Pressing
    // it made it current, so the page goes back to the tab shown before, and
    // the drop hint says which side the dragged tab would open on.
    property int draggedOutRow: -1
    function dragTabOverPage(index, scenePos) {
        if (index < 0) {
            panes.showDrop(null)
            if (draggedOutRow >= 0)
                Tabs.activate(draggedOutRow)
            draggedOutRow = -1
            return
        }
        if (draggedOutRow < 0) {
            draggedOutRow = index
            const previous = previousTab()
            if (previous >= 0)
                Tabs.activate(previous)
        }
        const split = JSON.parse(Tabs.currentSplit())
        panes.showDrop(split && split.tabs.length >= 4 ? null : panes.mapFromItem(null, scenePos))
    }

    // Let go over the page, the dragged tab opens beside the page shown, on
    // the side it was dropped.
    function dropTabOnPage(index, scenePos) {
        panes.showDrop(panes.mapFromItem(null, scenePos))
        const side = panes.dropSide
        panes.showDrop(null)
        draggedOutRow = -1
        if (side.length === 0 || index < 0 || index === Tabs.currentIndex) {
            if (index >= 0)
                Tabs.activate(index)
            return
        }
        // A full split view (four panes) takes no more; the page stays put.
        if (Tabs.splitTabs(Tabs.currentIndex, index) < 0)
            return
        if (side === "left")
            Tabs.movePane(index, -1)
        Tabs.activate(index)
    }

    // Ctrl/Cmd+Alt+S: put the current tab side by side with the tab used
    // before it (a new tab if there is none), or end its split view.
    function toggleSplit() {
        const split = JSON.parse(Tabs.currentSplit())
        if (split) {
            Tabs.endSplit(split.id)
            return
        }
        const current = Tabs.currentIndex
        const previous = previousTab()
        if (previous >= 0) {
            Tabs.splitTabs(current, previous)
        } else {
            const index = Tabs.openTabNextToCurrent("", false)
            Tabs.splitTabs(Tabs.currentIndex, index)
            Tabs.activate(index)
            window.focusUrlBar()
        }
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

    function showExtensions() {
        extensionsDialog.show()
    }

    // Window fullscreen, the same as F11: hides the toolbars too.
    function toggleFullScreen() {
        fullScreen.toggleWindow()
    }

    // A private window (a process of its own), with `target` in it if given.
    function openPrivateWindow(target) {
        Privacy.openPrivateWindow(target ? [target.toString()] : [])
    }

    function showProfiles() {
        profilesDialog.show()
    }

    function showImport() {
        importDialog.show()
    }

    // The password manager, its search set to `query` ("" for everything).
    // `section` is "passwords" (the default) or "addresses".
    function showPasswords(query, section) {
        passwordManager.show(query || "", section || "passwords")
    }

    function showPalette() {
        palette.show()
    }

    function showClearData() {
        clearDataDialog.show()
    }

    // Show one of Ion's own pages (`ion://settings/privacy` is "settings",
    // "privacy"): in the current tab if it is already there or blank, else
    // in a tab already showing it, else in a new tab.
    function openPage(id, path) {
        const target = Pages.url(id, path || "")
        const view = window.currentView
        if (view && (Pages.pageOf(view.url) === id || Basics.isNewTabUrl(view.url))) {
            if (view.url.toString() !== target)
                view.url = target
            return
        }
        for (let i = 0; i < Tabs.count; i++) {
            if (Tabs.workspaceAt(i) === Tabs.workspace && Pages.pageOf(Tabs.urlAt(i)) === id) {
                Tabs.activate(i)
                const tab = window.viewAt(i)
                if (path && tab)
                    tab.url = target
                return
            }
        }
        window.openTab(target)
    }

    // Served documents match the theme, so Ion's pages never flash white.
    Binding {
        target: Pages
        property: "background"
        value: Theme.background.toString()
    }

    // Keyboard focus follows the visible page (so Space, arrows and Find work
    // right after switching tabs), unless the person is typing in the URL bar.
    onCurrentViewChanged: Qt.callLater(focusPage)
    function focusPage() {
        if (currentView && !navBar.urlBar.activeFocus && !palette.opened && !tabSearch.opened)
            currentView.forceActiveFocus()
    }

    // Delete what `[privacy] clearOnExit` lists, one asynchronous profile
    // operation at a time, as Qt asks. At quit: cookies and history. At
    // start: history and the cache (the cookie database is already gone,
    // deleted in main.rs before the engine opened it). Returns whether a
    // cache clear started; pages wait for profile.clearHttpCacheCompleted.
    function clearOnExit(atStart) {
        const items = Privacy.clearOnExit()
        if (items.includes("cookies") && !atStart)
            Privacy.clearCookies(window.profile)
        if (items.includes("history")) {
            History.clear()
            History.save()
        }
        if (!atStart || !items.includes("cache"))
            return false
        window.profile.clearHttpCache()
        return true
    }

    // Set while startup waits for clearOnExit's cache clear to finish. No
    // timeout: Qt forbids navigating the profile before the clear completes.
    property bool waitingForCacheClear: false
    Connections {
        target: window.waitingForCacheClear ? window.profile : null
        function onClearHttpCacheCompleted() { window.finishStartup() }
    }

    // Extensions, then the restored session, then command-line URLs, else the
    // home page. Runs once the startup cache clear (if any) is done. Tab views
    // only exist once `startupFinished` is set at the end, so a tab opened
    // while waiting loads once, after the session restore.
    property bool startupFinished: false
    property bool startupRunning: false
    function finishStartup() {
        if (startupRunning)
            return
        startupRunning = true
        waitingForCacheClear = false
        // Chrome extensions from config and Ion's extensions folder.
        const extensionManager = window.profile.extensionManager
        if (extensionManager) {
            extensionPaths = Extensions.paths()
            extensionPaths.forEach(path => extensionManager.loadExtension(path))
        }
        // True when the last run crashed; its tabs are kept aside. A private
        // window starts empty, is never saved and leaves the record alone.
        const crashed = !window.privateWindow && Tabs.beginRun()
        if (Config.value("general.restoreSession") && !window.privateWindow) {
            // Restoring replaces every tab; keep any opened while waiting
            // (Ctrl+T, a URL handed over by a second launch).
            const early = []
            for (let i = 0; i < Tabs.count; i++)
                early.push(Tabs.urlAt(i))
            if (Tabs.restoreLastSession())
                early.forEach((target, i) => openTab(target, i === 0))
            Tabs.discardCrashedSession()
        } else if (crashed) {
            crashRestore.offer()
        }
        // URLs on the command line (`ion %U` from the desktop file) open as new
        // tabs after the restored ones; the first one becomes current.
        const urls = Qt.application.arguments.slice(1).filter(arg => !arg.startsWith("-"))
        urls.forEach((arg, i) => openTab(urlBarResolver.resolve(arg), i === 0))
        if (Tabs.count === 0)
            openTab(Pages.isFirstRun() ? Pages.url("welcome", "") : homeUrl)
        startupFinished = true
        sessionSaveTimer.restart()
    }

    function searchTabs() { tabSearch.show() }
    function focusUrlBar() {
        navBar.urlBar.forceActiveFocus()
        navBar.urlBar.selectAll()
    }

    Component.onCompleted: {
        profile = profilePrototype.instance()
        Adblock.attach(window.profile)
        Pages.attach(window.profile)
        Privacy.apply(window.profile)
        // Again at start, in case the last quit was cut short; the rest of
        // startup waits for it. A private window has nothing to clear.
        if (!window.privateWindow && clearOnExit(true))
            waitingForCacheClear = true
        else
            finishStartup()
    }

    // Extensions load switched off; turn on each one that loaded cleanly,
    // unless it was switched off in the Extensions dialog.
    Connections {
        target: window.profile?.extensionManager ?? null
        ignoreUnknownSignals: true
        function onLoadFinished(extension) {
            if (extension.isLoaded && !extension.isEnabled && extension.error.length === 0
                    && !Extensions.isDisabled(extension.path.toString()))
                window.profile.extensionManager.setExtensionEnabled(extension, true)
        }
    }

    // Save tabs shortly after they change, and history less eagerly; both are
    // written once more on quit.
    Timer {
        id: sessionSaveTimer
        interval: 1000
        onTriggered: Tabs.saveSession()
    }
    // Unload background tabs nobody has looked at for `seconds` (0: every
    // background tab). Tabs on screen, playing sound or waiting with a dialog
    // are left alone.
    function sleepIdleTabs(seconds) {
        for (const index of Tabs.idleTabs(seconds)) {
            const view = views.itemAt(index)
            if (view && !view.recentlyAudible && !view.needsAttention)
                Tabs.setSuspended(index, true)
        }
    }
    // Every minute, for tabs idle for `general.suspendTabsAfter` minutes.
    Timer {
        interval: 60 * 1000
        repeat: true
        running: true
        onTriggered: {
            const minutes = Config.value("general.suspendTabsAfter")
            if (minutes > 0)
                window.sleepIdleTabs(minutes * 60)
        }
    }
    Timer {
        id: historySaveTimer
        interval: 5000
        onTriggered: History.save()
    }
    Connections {
        target: Tabs
        // Not before startup restores the saved session, or it would be
        // overwritten by tabs opened while waiting.
        function onSessionChanged() {
            if (window.startupFinished && !window.privateWindow)
                sessionSaveTimer.restart()
        }
    }
    Connections {
        target: History
        function onChanged() { historySaveTimer.restart() }
    }
    // `[privacy]` cookie settings follow config changes live.
    Connections {
        target: Config
        function onRevisionChanged() { Privacy.apply(window.profile) }
    }
    Connections {
        target: Qt.application
        function onAboutToQuit() {
            // Before startup restores it, the tab list would overwrite the
            // saved session.
            if (window.privateWindow)
                return
            if (window.startupFinished)
                Tabs.saveSession()
            Tabs.endRun()
            History.save()
            window.clearOnExit(false)
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
    // Tab search (Ctrl/Cmd+Shift+A).
    TabSearch { id: tabSearch; browser: window }

    ImportDialog { id: importDialog }
    PasswordManager { id: passwordManager }
    ProfilesDialog { id: profilesDialog }
    ClearDataDialog { id: clearDataDialog; profile: window.profile }
    ExtensionsDialog { id: extensionsDialog; browser: window }

    // Per-site CSS and userscripts, installed on the profile.
    SiteScripts { profile: window.profile }

    header: ColumnLayout {
        spacing: 0
        visible: !fullScreen.active

        TabStrip {
            Layout.fillWidth: true
            visible: !window.verticalTabs
            tabs: Tabs
            currentIndex: Tabs.currentIndex
            workspace: Tabs.workspace
            onActivated: index => Tabs.activate(index)
            onCloseRequested: index => window.closeTab(index)
            onMoveRequested: (from, to) => Tabs.moveTab(from, to)
            onTabDraggedOut: (index, pos) => window.dragTabOverPage(index, pos)
            onTabDroppedOut: (index, pos) => window.dropTabOnPage(index, pos)
            onNewTabRequested: window.newTab()
            onSearchRequested: tabSearch.show()
            onMenuRequested: anchor => sessionMenu.open(anchor)
            onTabMenuRequested: index => tabMenu.openFor(index)
            onGroupMenuRequested: (group, anchor) => groupMenu.openFor(group, anchor)
            onWorkspaceMenuRequested: anchor => workspaceMenu.open(anchor)
            views: views
            viewsRevision: window.viewsRevision
            currentSplit: panes.split ? panes.split.id : -1
        }

        NavigationBar {
            id: navBar
            Layout.fillWidth: true
            view: window.currentView
            downloads: downloads
            agentSidebar: agentSidebar
            onBookmarkToggled: window.toggleBookmark()
            onPasswordsRequested: query => window.showPasswords(query)
            onProfilesRequested: window.showProfiles()
        }
    }

    SessionMenu {
        id: sessionMenu
        onSearchRequested: tabSearch.show()
        onSleepRequested: window.sleepIdleTabs(0)
    }
    TabMenu { id: tabMenu; browser: window; workspaceMenu: workspaceMenu; groupMenu: groupMenu }
    GroupMenu { id: groupMenu }
    WorkspaceMenu { id: workspaceMenu }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        VerticalTabStrip {
            Layout.fillHeight: true
            // Above the page, which an expanded collapsed sidebar covers.
            z: 1
            visible: window.verticalTabs && !fullScreen.active
            collapsed: {
                Config.revision
                return Config.value("ui.collapseSidebar") === true
            }
            tabs: Tabs
            currentIndex: Tabs.currentIndex
            workspace: Tabs.workspace
            onActivated: index => Tabs.activate(index)
            onCloseRequested: index => window.closeTab(index)
            onMoveRequested: (from, to) => Tabs.moveTab(from, to)
            onTabDraggedOut: (index, pos) => window.dragTabOverPage(index, pos)
            onTabDroppedOut: (index, pos) => window.dropTabOnPage(index, pos)
            onNewTabRequested: window.newTab()
            onSearchRequested: tabSearch.show()
            onMenuRequested: anchor => sessionMenu.open(anchor)
            onTabMenuRequested: index => tabMenu.openFor(index)
            onGroupMenuRequested: (group, anchor) => groupMenu.openFor(group, anchor)
            onWorkspaceMenuRequested: anchor => workspaceMenu.open(anchor)
            views: views
            viewsRevision: window.viewsRevision
            currentSplit: panes.split ? panes.split.id : -1
        }

        // One web view per tab, stacked; only the current one is visible, or
        // the panes of its split view.
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            // Above the views: the current pane's outline, resize handles and
            // the drop hint for tabs dragged onto the page.
            SplitPanes { id: panes; anchors.fill: parent; z: 1 }

            Repeater {
                id: views
                // No page loads before startup's cache clear is done; tabs
                // opened meanwhile get their views then.
                model: window.startupFinished ? Tabs : null

                onItemAdded: window.viewsRevision++
                onItemRemoved: window.viewsRevision++

                delegate: BrowserTab {
                    id: view

                    required property int index
                    required property bool suspended
                    readonly property bool current: index === Tabs.currentIndex
                    readonly property bool inSplit: panes.paneOf(index) >= 0
                    readonly property rect pane: panes.rectFor(index, panes.width, panes.height, panes.sizes)
                    // Background tabs restored from a session load when first shown.
                    property bool deferred: false

                    function loadSavedUrl() {
                        const target = Tabs.urlAt(index)
                        if (target.length > 0)
                            url = target
                    }

                    x: pane.x
                    y: pane.y
                    width: pane.width
                    height: pane.height
                    visible: current || inSplit
                    profile: window.profile
                    // Clicking into a pane of a split view makes its tab current;
                    // the click still reaches the page.
                    MouseArea {
                        anchors.fill: parent
                        z: 1
                        enabled: view.inSplit && !view.current
                        acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
                        onPressed: mouse => {
                            Tabs.activate(view.index)
                            mouse.accepted = false
                        }
                    }
                    // A suspended tab's page is unloaded; it reloads when shown.
                    lifecycleState: suspended ? WebEngineView.LifecycleState.Discarded
                                              : WebEngineView.LifecycleState.Active

                    // The current tab, or a pane of the split view showing.
                    readonly property bool onScreen: current || inSplit

                    Component.onCompleted: {
                        deferred = Tabs.restoring && !onScreen
                        if (!deferred)
                            loadSavedUrl()
                    }
                    onOnScreenChanged: {
                        if (onScreen && deferred) {
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
                        // The new-tab page has no title of its own; the strip says "New Tab".
                        Tabs.setTitle(index, showNewTabPage && Basics.isNewTabUrl(url) ? "" : title)
                        if (!window.privateWindow)
                            History.updateTitle(url.toString(), title)
                    }
                    onIconChanged: {
                        if (!deferred)
                            Tabs.setIcon(index, icon.toString())
                    }
                    onLoadingChanged: info => {
                        if (info.status === WebEngineView.LoadSucceededStatus && !window.privateWindow)
                            History.recordVisit(info.url.toString(), title)
                    }
                    onNewTabRequested: request => {
                        const background = request.destination === WebEngineNewWindowRequest.InNewBackgroundTab
                        const tab = window.openPageTab(!background)
                        tab.showNewTabPage = false
                        tab.openerTabId = view.tabId
                        request.openIn(tab)
                    }
                    // A page a script opened may close itself (window.close()),
                    // like a "Sign in with Google" or PayPal popup once it's
                    // done; the tab goes and its opener comes back.
                    property int openerTabId: -1
                    onWindowCloseRequested: {
                        const opener = Tabs.indexOfTab(openerTabId)
                        const wasCurrent = current
                        window.closeTab(index)
                        if (wasCurrent && opener >= 0)
                            Tabs.activate(Tabs.indexOfTab(openerTabId))
                    }
                    onOpenInNewTab: target => window.openTab(target, false)
                    onAskAgent: input => window.askAgent(input)
                    onManagePasswords: section => window.showPasswords(section === "addresses" ? "" : Passwords.host(url.toString()), section)
                }
            }
        }

        AgentSidebar {
            id: agentSidebar
            Layout.fillHeight: true
            Layout.preferredWidth: implicitWidth
            view: window.currentView
            onAsk: input => navBar.askAgent(input)
            onActivityRequested: agentActivity.show()
            onFinished: window.currentView?.forceActiveFocus()
        }
    }

    DownloadsPanel { id: downloads; profile: window.profile; browser: window }
    NotificationToasts { profile: window.profile }
    FullScreenController {
        id: fullScreen
        window: window
        view: window.currentView
        // Esc leaves page fullscreen; an open downloads panel would make the
        // two Esc shortcuts ambiguous so neither fired.
        onPageActiveChanged: if (pageActive) downloads.close()
    }

    // Every command, from its shortcut, the palette or anywhere else; keys
    // come from `Keymap` ("Ctrl" is Cmd on macOS). Stopping every agent
    // (docs/SAFETY.md) is one of them; resuming is deliberate: the notice's
    // button or the palette, never the same key again.
    Commands {
        id: commands
        browser: window
        commandPalette: palette
        shortcutEditor: shortcutEditor
        downloads: downloads
    }
    function runCommand(id) { commands.run(id) }
    function canRunCommand(id) { return commands.available(id) }

    ShortcutEditor { id: shortcutEditor }

    // Agents, from shortcuts, the palette and the page's menu.
    function askAgent(input) {
        navBar.askAgent(input)
    }
    // The address bar, ready with "@ion ".
    function startAgentQuestion() {
        navBar.urlBar.forceActiveFocus()
        navBar.urlBar.text = "@ion "
        navBar.urlBar.cursorPosition = navBar.urlBar.text.length
    }
    function showAgentActivity() {
        agentActivity.show()
    }
    function showAgentSetup() {
        agentSetup.show("")
    }
    function toggleAgentSidebar() {
        agentSidebar.toggle(window.currentView ? Agent.latestTask(window.currentView.tabId) : -1)
    }

    AgentsStoppedNotice {}
    CrashRestoreNotice { id: crashRestore }
    AgentActivity { id: agentActivity }
    AgentSetup {
        id: agentSetup
        onAsk: input => window.askAgent(input)
    }
    AgentTools { views: views }
}
