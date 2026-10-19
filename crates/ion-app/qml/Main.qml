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
    // Opening it starts Chromium's storage and network service, the slowest
    // step of startup, so it is created in startEngine() once the window has
    // drawn its first frame rather than while the window is being built.
    Component {
        id: profileComponent
        WebEngineProfilePrototype {
            // "Default", a profile's own, or "" (off the record) in a private window.
            storageName: Privacy.storageName()
            // Site permissions are decided and remembered by Ion's safety model
            // (Safety, docs/SAFETY.md), so the engine keeps none of its own.
            persistentPermissionsPolicy: WebEngineProfile.PersistentPermissionsPolicy.AskEveryTime
        }
    }
    property WebEngineProfilePrototype profilePrototype: null
    // Null until startEngine().
    property WebEngineProfile profile: null
    // The downloads panel's requests, newest first (the Downloads page lists
    // the running ones).
    readonly property var downloadItems: downloads.items
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
        // Essentials never join a split, so they get no drop hint either.
        if (Tabs.pinnedAt(index) === 2)
            return
        if (draggedOutRow < 0) {
            draggedOutRow = index
            const previous = previousTab()
            if (previous >= 0)
                Tabs.activate(previous)
        }
        // No hint where the drop can't split: a full split view the tab isn't
        // already in, or an Essential shown in place of the dragged tab.
        const split = JSON.parse(Tabs.currentSplit())
        const full = split && split.tabs.length >= 4 && !split.tabs.includes(index)
        panes.showDrop(full || Tabs.pinnedAt(Tabs.currentIndex) === 2 ? null : panes.mapFromItem(null, scenePos))
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
        // A full split view (four panes) takes no more, and an Essential joins
        // none; the dragged tab stays current as it was before the drag.
        if (Tabs.splitTabs(Tabs.currentIndex, index) < 0) {
            Tabs.activate(index)
            return
        }
        // Next to the current pane, on the side the hint showed; a tab already
        // in the split moves there too.
        Tabs.placePane(index, Tabs.currentIndex, side === "right" || side === "bottom")
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
        // Essentials show in every workspace, so they never share the page.
        if (Tabs.pinnedAt(current) === 2)
            return
        const previous = previousTab()
        if (previous >= 0 && Tabs.pinnedAt(previous) !== 2) {
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
        extensionsDialog.get().show()
    }

    // Window fullscreen, the same as F11: hides the toolbars too.
    function toggleFullScreen() {
        fullScreen.toggleWindow()
    }

    // Mute or unmute the current tab (Ctrl+Shift+M, the tab's speaker).
    function toggleMute() {
        const view = window.currentView
        if (view)
            view.audioMuted = !view.audioMuted
    }

    // Silence every tab but the current one, wherever they are.
    function muteOtherTabs() {
        for (let i = 0; i < views.count; i++) {
            const view = views.itemAt(i)
            if (view && i !== Tabs.currentIndex)
                view.audioMuted = true
        }
    }

    // A private window (a process of its own), with `target` in it if given.
    function openPrivateWindow(target) {
        Privacy.openPrivateWindow(target ? [target.toString()] : [])
    }

    function showProfiles() {
        profilesDialog.get().show()
    }

    function showImport() {
        importDialog.get().show()
    }

    // The password manager, its search set to `query` ("" for everything).
    // `section` is "passwords" (the default) or "addresses".
    function showPasswords(query, section) {
        passwordManager.get().show(query || "", section || "passwords")
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
    onCurrentViewChanged: {
        Qt.callLater(focusPage)
        autoPictureInPicture()
    }
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
    // True while the views of a restored session are created; see finishStartup.
    property bool restoringViews: false
    // Restored tabs on screen wait too: a startup tab is about to replace them.
    property bool restoreDisplaced: false
    // This launch has marked itself running (Tabs.beginRun).
    property bool runStarted: false
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
        runStarted = !window.privateWindow
        // Restoring replaces every tab; tabs opened while waiting (Ctrl+T, a
        // URL handed over by a second launch) are opened again after it.
        let early = []
        let restored = false
        if (Config.value("general.restoreSession") && !window.privateWindow) {
            for (let i = 0; i < Tabs.count; i++)
                early.push(Tabs.urlAt(i))
            restored = Tabs.restoreLastSession()
            if (!restored)
                early = []
            Tabs.discardCrashedSession()
        } else if (crashed) {
            crashRestore.offer()
        }
        // URLs on the command line (`ion %U` from the desktop file) open as new
        // tabs after the restored ones; the first one becomes current.
        const urls = Qt.application.arguments.slice(1).filter(arg => !arg.startsWith("-"))
        // The tab views are created now. Restored background tabs wait until
        // they are first shown, so a big session costs little at startup;
        // tabs opened below load right away. When one of those takes over the
        // screen, the restored tabs it replaces wait too.
        restoringViews = restored
        restoreDisplaced = restored && (early.length > 0 || urls.length > 0)
        startupFinished = true
        restoringViews = false
        early.forEach((target, i) => openTab(target, i === 0))
        urls.forEach((arg, i) => openTab(urlBarResolver.resolve(arg), i === 0))
        if (restoreDisplaced) {
            restoreDisplaced = false
            // Whatever is still on screen loads now.
            for (let i = 0; i < views.count; i++) {
                const view = views.itemAt(i)
                if (view && view.onScreen && view.deferred) {
                    view.deferred = false
                    view.loadSavedUrl()
                }
            }
        }
        if (Tabs.count === 0)
            openTab(Pages.isFirstRun() ? Pages.url("welcome", "") : homeUrl)
        sessionSaveTimer.restart()
    }

    function searchTabs() { tabSearch.show() }
    function focusUrlBar() {
        navBar.urlBar.forceActiveFocus()
        navBar.urlBar.selectAll()
    }

    // Startup marks for bench/bench.py; Performance.mark keeps each once.
    property bool firstFrameShown: false
    Connections {
        target: window
        enabled: !window.firstFrameShown
        function onFrameSwapped() {
            window.firstFrameShown = true
            Performance.mark("first-frame")
        }
    }

    // The window shows first; the engine starts right after its first frame,
    // or after a moment if the window isn't drawn (started hidden).
    Timer {
        id: engineFallback
        interval: 200
        running: true
        onTriggered: window.startEngine()
    }
    onFirstFrameShownChanged: Qt.callLater(startEngine)

    function startEngine() {
        if (profilePrototype)
            return
        engineFallback.stop()
        profilePrototype = profileComponent.createObject(window)
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
            if (view && !view.recentlyAudible && !view.needsAttention && !view.pictureInPicture)
                Tabs.setSuspended(index, true)
        }
    }
    // Every minute, for tabs idle for `general.suspendTabsAfter` minutes, or
    // sooner while the system runs short of memory: after five minutes under
    // moderate pressure, at once under critical pressure (as Chrome and
    // Safari do, so the system doesn't start killing processes). 0 keeps
    // every tab awake regardless.
    function sleepForPressure() {
        const minutes = Config.value("general.suspendTabsAfter")
        if (!(minutes > 0))
            return
        const pressure = Performance.memoryPressure
        if (pressure === "critical")
            sleepIdleTabs(0)
        else if (pressure === "moderate")
            sleepIdleTabs(Math.min(minutes, 5) * 60)
        else
            sleepIdleTabs(minutes * 60)
    }
    Timer {
        interval: 60 * 1000
        repeat: true
        running: true
        onTriggered: window.sleepForPressure()
    }
    Connections {
        target: Performance
        function onMemoryPressureChanged() {
            if (Performance.memoryPressure !== "normal")
                window.sleepForPressure()
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
        // A private window shares the data directory; the normal window's
        // process owns history.json and inputs.json.
        function onChanged() {
            if (!window.privateWindow)
                historySaveTimer.restart()
        }
    }
    // `[privacy]` cookie settings follow config changes live.
    Connections {
        target: Config
        function onRevisionChanged() {
            if (window.profile)
                Privacy.apply(window.profile)
        }
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
            // Quitting before beginRun() keeps the last run's crash record.
            if (window.runStarted)
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

    // Two-finger swipes go back and forward; pinching zooms the page; with
    // `gestures.mouse`, right-button strokes run commands.
    Gestures {
        id: gestures
        area: {
            pageArea.x; pageArea.y; pageArea.width; pageArea.height
            return pageArea.mapToItem(null, 0, 0, pageArea.width, pageArea.height)
        }
        readonly property var view: window.currentView
        canGoBack: !!view && view.canGoBack && view.scrollPosition.x <= 1
        // scrollPosition and contentsSize are already zoomed, like width.
        canGoForward: !!view && view.canGoForward
            && view.scrollPosition.x + view.width >= view.contentsSize.width - 1
        zoomFactor: view ? view.zoomFactor : 1
        onNavigate: direction => direction === "back" ? view?.goBack() : view?.goForward()
        // A carousel or editor under the fingers that can still scroll
        // sideways gets the gesture instead.
        onSwipeStarted: (gesture, x, y) => {
            if (!view) {
                block(gesture, false, false)
                return
            }
            // A popup over the page (downloads, menus) keeps the gesture.
            for (const item of window.Overlay.overlay?.children ?? []) {
                if (item.visible && item.contains(item.mapFromItem(null, x, y))) {
                    block(gesture, true, true)
                    return
                }
            }
            const point = view.mapFromItem(null, x, y)
            view.runJavaScript(`(function check(doc, x, y) {
                let back = false, forward = false
                // Whether a box can still scroll either way. Right-to-left
                // boxes count scrollLeft from the right edge, going negative.
                const room = (e, style) => {
                    const max = e.scrollWidth - e.clientWidth
                    const left = style.direction === "rtl" ? max + e.scrollLeft : e.scrollLeft
                    back = back || left > 0
                    forward = forward || left < max - 1
                }
                // The innermost element, inside open shadow roots too.
                let e = doc.elementFromPoint(x, y)
                while (e && e.shadowRoot) {
                    const inner = e.shadowRoot.elementFromPoint(x, y)
                    if (!inner || inner === e)
                        break
                    e = inner
                }
                for (; e && e !== doc.body && e !== doc.documentElement;
                        e = e.parentElement ?? e.getRootNode().host) {
                    if (e.tagName === "IFRAME" || e.tagName === "FRAME") {
                        // Look inside a same-origin frame; a cross-origin
                        // one can't be seen into, so it keeps the gesture.
                        let inner = null
                        try { inner = e.contentDocument } catch (error) {}
                        if (!inner)
                            return [true, true]
                        const box = e.getBoundingClientRect()
                        const found = check(inner, x - box.left - e.clientLeft, y - box.top - e.clientTop)
                        back = back || found[0]
                        forward = forward || found[1]
                        continue
                    }
                    if (e.scrollWidth - e.clientWidth <= 1)
                        continue
                    const style = doc.defaultView.getComputedStyle(e)
                    if (style.overflowX === "auto" || style.overflowX === "scroll")
                        room(e, style)
                }
                // A frame's own page scrolls too (the top page's doesn't
                // count: Ion checks its edges itself).
                const page = doc !== document && doc.scrollingElement
                if (page && page.scrollWidth - page.clientWidth > 1)
                    room(page, doc.defaultView.getComputedStyle(page))
                return [back, forward]
            })(document, ${point.x / view.zoomFactor}, ${point.y / view.zoomFactor})`,
                WebEngineScript.ApplicationWorld, result => {
                    // Arrives as a list, not a JS Array.
                    const found = result && result.length === 2 ? [!!result[0], !!result[1]] : [false, false]
                    block(gesture, found[0], found[1])
                })
        }
        onZoom: (factor, done) => {
            if (!view)
                return
            if (done)
                view.setZoom(factor)
            else
                view.zoomFactor = factor
        }
        onCommand: id => window.runCommand(id)
        // In a split view the stroke acts on the pane it starts in.
        onStrokeStarted: (x, y) => {
            for (let i = 0; i < views.count; i++) {
                const pane = views.itemAt(i)
                if (pane && pane.inSplit && !pane.current
                        && pane.contains(pane.mapFromItem(null, x, y))) {
                    Tabs.activate(pane.index)
                    return
                }
            }
        }
        onStrokeMoved: (x, y) => strokeTrail.add(x, y)
        Component.onCompleted: attach(window)
    }

    // Command palette (Ctrl/Cmd+K).
    CommandPalette { id: palette; browser: window }
    // Tab search (Ctrl/Cmd+Shift+A).
    TabSearch { id: tabSearch; browser: window }
    SiteSearchDiscovery { id: siteSearch }

    // Picture-in-picture: a tab's video in its own small window.
    PictureInPicture {
        id: pipWindow
        home: views.parent
        browser: window
    }
    function togglePictureInPicture() {
        if (pipWindow.active)
            pipWindow.restore(false)
        else
            pipWindow.open(window.currentView, false)
    }

    // Switching away from a tab playing a video pops it out (and back when
    // you return), with `media.autoPictureInPicture`.
    property var previousView: null
    function autoPictureInPicture() {
        const left = previousView
        previousView = currentView
        if (pipWindow.automatic && currentView && pipWindow.view === currentView) {
            pipWindow.restore(false)
            return
        }
        // Back before its video popped out: it stays put.
        if (currentView && pipWindow.pendingTab === currentView)
            pipWindow.forget(currentView)
        if (left && left !== currentView && !left.pictureInPicture && left.recentlyAudible
                && !left.audioMuted && Config.value("media.autoPictureInPicture") !== false)
            pipWindow.open(left, true)
    }

    // What tabs are playing, for the now-playing button and the system's
    // media controls.
    MediaSessions { id: mediaSessions; browser: window; pip: pipWindow }

    // Print preview (Ctrl+P, window.print(), the PDF viewer's print button).
    PrintPreview { id: printPreview }
    // `saveAsPdf` starts on "Save as PDF" rather than the default printer.
    function printPage(saveAsPdf) {
        // The page, not just its video.
        if (pipWindow.view && pipWindow.view === window.currentView)
            pipWindow.restore(false)
        printPreview.open(window.currentView, saveAsPdf === true)
    }

    // Dialogs are built the first time they open (Lazy.get()).
    Lazy { id: importDialog; sourceComponent: ImportDialog {} }
    Lazy { id: passwordManager; sourceComponent: PasswordManager {} }
    Lazy { id: profilesDialog; sourceComponent: ProfilesDialog {} }
    ClearDataDialog { id: clearDataDialog; profile: window.profile }
    Lazy {
        id: extensionsDialog
        sourceComponent: ExtensionsDialog { browser: window }
    }

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

        // With horizontal tabs the toolbar spans the window under them.
        LayoutItemProxy {
            Layout.fillWidth: true
            visible: !window.verticalTabs
            target: navBar
        }
    }

    // One toolbar, placed by a LayoutItemProxy: under the tab strip, or over
    // the page beside the vertical tab sidebar, which then runs full height.
    NavigationBar {
        id: navBar
        view: window.currentView
        downloads: downloads
        mediaSessions: mediaSessions
        agentSidebar: agentSidebar
        onBookmarkToggled: window.toggleBookmark()
        onRunCommand: id => window.runCommand(id)
        onPasswordsRequested: query => window.showPasswords(query)
        onProfilesRequested: window.showProfiles()
    }

    SessionMenu {
        id: sessionMenu
        onSearchRequested: tabSearch.show()
        onSleepRequested: window.sleepIdleTabs(0)
    }
    TabMenu { id: tabMenu; browser: window; workspaceMenu: workspaceMenu; groupMenu: groupMenu }
    GroupMenu { id: groupMenu }
    WorkspaceMenu { id: workspaceMenu }

    // Sidebars run full height on either side; the middle column holds the
    // toolbar (vertical tabs only) over the page.
    GridLayout {
        anchors.fill: parent
        columns: 4
        rowSpacing: 0
        columnSpacing: 0

        VerticalTabStrip {
            Layout.row: 0
            Layout.column: 0
            Layout.rowSpan: 2
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

        LayoutItemProxy {
            Layout.row: 0
            Layout.column: 1
            Layout.fillWidth: true
            visible: window.verticalTabs && !fullScreen.active
            target: navBar
        }

        // One web view per tab, stacked; only the current one is visible, or
        // the panes of its split view. The page sits in the chrome as a
        // rounded card, edge to edge in fullscreen.
        Item {
            id: pageArea

            readonly property bool framed: !fullScreen.active
            Layout.row: 1
            Layout.column: 1
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: framed && !window.verticalTabs ? Theme.spacing * 1.5 : 0
            Layout.rightMargin: framed ? Theme.spacing * 1.5 : 0
            Layout.bottomMargin: framed ? Theme.spacing * 1.5 : 0

            IonShadow {
                anchors.fill: parent
                visible: pageArea.framed
                radius: Theme.radiusLarge
                spread: Theme.spacing * 1.5
                offset: Theme.hairline
                strength: 0.5
            }
            PageFrame {
                anchors.fill: parent
                z: 100
                visible: pageArea.framed
            }

            // Shown in place of a tab whose video is in picture-in-picture.
            Column {
                anchors.centerIn: parent
                spacing: Theme.spacing * 2
                visible: window.currentView?.pictureInPicture ?? false

                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: qsTr("Playing in picture-in-picture")
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSize + 2
                }
                DialogButton {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: qsTr("Bring it back")
                    onClicked: pipWindow.restore(false)
                }
            }

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

                    // A tab in picture-in-picture fills that window instead.
                    x: pictureInPicture ? 0 : pane.x
                    y: pictureInPicture ? 0 : pane.y
                    width: pictureInPicture ? parent.width : pane.width
                    height: pictureInPicture ? parent.height : pane.height
                    visible: current || inSplit || pictureInPicture
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
                        deferred = (Tabs.restoring || window.restoringViews)
                                && (!onScreen || window.restoreDisplaced)
                        if (!deferred)
                            loadSavedUrl()
                    }
                    Component.onDestruction: {
                        pipWindow.forget(view)
                        mediaSessions.update(view, true)
                    }
                    onMediaChanged: mediaSessions.update(view, false)
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
                        if (info.status === WebEngineView.LoadSucceededStatus) {
                            // Private windows leave no history and learn no site searches.
                            if (!window.privateWindow) {
                                History.recordVisit(info.url.toString(), title)
                                siteSearch.check(view, info.url)
                            }
                            Performance.mark("first-page")
                            Performance.event("page-loaded")
                        }
                    }
                    onNewTabRequested: request => {
                        const background = request.destination === WebEngineNewWindowRequest.InNewBackgroundTab
                        const tab = window.openPageTab(!background)
                        tab.showNewTabPage = false
                        tab.openerTabId = view.tabId
                        // An adopted page never navigates here, so it takes
                        // its site's autoplay policy now; a blank one is the
                        // opener's.
                        const target = request.requestedUrl.toString()
                        const blank = target.length === 0 || target.startsWith("about:")
                        tab.autoplayInherited = blank ? view.autoplayUrl() : ""
                        tab.applyAutoplay(blank ? tab.autoplayInherited : target)
                        request.openIn(tab)
                    }
                    onPictureInPictureRequested: enter => {
                        if (enter)
                            pipWindow.open(view, false)
                        else if (pipWindow.view === view)
                            pipWindow.restore(false)
                    }
                    onPrintRequested: printPreview.open(view)
                    onPrintRequestedByFrame: printPreview.open(view)
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

        // Notes and web panels (Ctrl+Shift+L).
        SidePanel {
            id: sidePanel
            // Beside the toolbar too only when the toolbar row is in use
            // (vertical tabs); spanning an empty row would open it up.
            Layout.row: window.verticalTabs ? 0 : 1
            Layout.column: 2
            Layout.rowSpan: window.verticalTabs ? 2 : 1
            Layout.fillHeight: true
            Layout.preferredHeight: 0
            Layout.preferredWidth: implicitWidth
            browser: window
        }

        AgentSidebar {
            id: agentSidebar
            Layout.row: window.verticalTabs ? 0 : 1
            Layout.column: 3
            Layout.rowSpan: window.verticalTabs ? 2 : 1
            Layout.fillHeight: true
            Layout.preferredHeight: 0
            Layout.preferredWidth: implicitWidth
            view: window.currentView
            onAsk: input => navBar.askAgent(input)
            onActivityRequested: agentActivity.get().show()
            onFinished: window.currentView?.forceActiveFocus()
        }
    }

    StrokeTrail {
        id: strokeTrail
        parent: pageArea
        active: gestures.stroking
        label: gestures.strokeLabel
    }
    SwipeIndicator {
        parent: pageArea
        direction: gestures.swipeDirection
        progress: gestures.swipeProgress
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
    // What the side-panel command reopens when nothing is showing.
    function lastPanel() { return sidePanel.lastOpen }
    function copyText(text) { commands.copyText(text) }
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
        agentActivity.get().show()
    }
    function showAgentSetup() {
        agentSetup.get().show("")
    }
    function toggleAgentSidebar() {
        agentSidebar.toggle(window.currentView ? Agent.latestTask(window.currentView.tabId) : -1)
    }

    AgentsStoppedNotice {}
    CrashRestoreNotice { id: crashRestore }
    Lazy { id: agentActivity; sourceComponent: AgentActivity {} }
    Lazy {
        id: agentSetup
        sourceComponent: AgentSetup { onAsk: input => window.askAgent(input) }
    }
    // Here, not in AgentSetup: the dialog doesn't exist until it first opens.
    Connections {
        target: Agent
        function onSetupRequested(input) { agentSetup.get().show(input) }
    }
    AgentTools { views: views }
}
