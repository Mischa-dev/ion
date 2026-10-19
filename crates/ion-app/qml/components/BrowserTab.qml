import QtQuick
import QtWebEngine
import Ion

// One tab's web content. Thin wrapper over WebEngineView so per-tab features
// (adblock counters, agent indicators, per-site settings) have a home.
WebEngineView {
    id: view

    // The tab's id in the `Tabs` model, stable while the tab is open.
    required property int tabId

    // Reader mode for this tab: `reader.toggle()`, `reader.active`.
    readonly property alias reader: readerMode
    // Saved passwords on this tab's pages: `passwords.prompt`, `passwords.formOnPage`.
    readonly property alias passwords: formFill
    readonly property alias findBar: findBar
    // Page translation: `translator.translate()`, `translator.showOriginal()`.
    readonly property alias translator: translator
    // A dialog (alert, sign-in…) is waiting for this tab to be shown.
    readonly property bool needsAttention: pageDialog.item ? pageDialog.item.waiting : false
    // Ion's own page drawn over this tab, if any (`builtinPage.active`).
    readonly property alias builtinPage: builtinPage
    // Ion's error page is showing: the address didn't load, whatever its scheme.
    readonly property bool loadFailed: loadError.item?.visible ?? false

    // Emitted when the page asks for a new tab or window (target=_blank, window.open).
    signal newTabRequested(var request)
    // Emitted when Ion's own UI (context menu) wants `target` opened in a new tab.
    signal openInNewTab(url target)
    // The page's menu asked an agent about the selection (`@ion …`).
    signal askAgent(string input)
    // The page's own picture-in-picture button (requestPictureInPicture())
    // or exitPictureInPicture().
    signal pictureInPictureRequested(bool enter)

    // In Ion's picture-in-picture window rather than the tab stack.
    property bool pictureInPicture: false

    // What the page is playing (Media.sessionReport's state: playing,
    // title, artist, artwork, …), or null. One entry per reporting frame in
    // `mediaFrames`; `media` is the one that matters most.
    property var media: null
    property var mediaFrames: ({})
    // The id of the frame `media` came from; actions go only there.
    property string mediaFrame: ""

    // A login field's list asked for the password manager.
    signal managePasswords(string section)

    // False for tabs a page opened (window.open), whose blank document the
    // page fills in itself; those must not be covered by the new-tab page.
    property bool showNewTabPage: true

    // Becomes true, for good, the first time the tab is shown or given a
    // page. The per-page UI below (new-tab page, prompts, dialogs, error page…)
    // is only created then, so restored background tabs that haven't been
    // looked at yet cost little more than a bare web view.
    property bool live: false
    // The parts' own `view` properties hide this file's `view` id.
    readonly property WebEngineView browserTab: view
    onVisibleChanged: {
        if (visible) {
            live = true
            snapshotSoon.restart()
        }
    }

    // A tab with no page yet matches the browser chrome instead of glaring
    // white. Pages get the web's usual white canvas, which many rely on.
    backgroundColor: url.toString().length === 0 || url.toString() === "about:blank"
            || Pages.pageOf(url).length > 0
        ? Theme.background : Theme.pageCanvas

    // Pages follow Ion's theme (theme.pages, [theme.sites]). QtWebEngine reads
    // the light/dark preference and force-dark only when a page's settings
    // are applied, so setting forceDarkMode (even to the same value)
    // re-applies them.
    function applyPageTheme() {
        settings.forceDarkMode = Theme.engine.darkenPage(url.toString())
        applySiteCss()
    }

    // Page and per-site CSS come from one user script that matches hosts
    // itself. It runs on every new document, frames included; running it in
    // each frame now updates the open page.
    property string siteCssScript: ""
    function applySiteCss() {
        const script = Theme.engine.siteCssScript
        if (script === siteCssScript)
            return
        const old = userScripts.find("ion-site-css")
        if (old.length > 0)
            userScripts.remove(old[0])
        if (script.length > 0) {
            const userScript = WebEngine.script()
            userScript.name = "ion-site-css"
            userScript.sourceCode = script
            userScript.injectionPoint = WebEngineScript.DocumentCreation
            userScript.worldId = WebEngineScript.ApplicationWorld
            userScript.runsOnSubFrames = true
            userScripts.insert(userScript)
            runInFrames(mainFrame, script)
        } else {
            runInFrames(mainFrame, Theme.engine.clearSiteCssScript())
        }
        siteCssScript = script
    }
    // `script` in `frame` and all its subframes, in Ion's world unless
    // `world` says otherwise.
    function runInFrames(frame, script, world) {
        const target = world ?? WebEngineScript.ApplicationWorld
        frame.runJavaScript(script, target)
        for (const child of frame.children)
            runInFrames(child, script, target)
    }

    Component.onCompleted: {
        if (visible || url.toString().length > 0)
            live = true
        applyPageTheme()
        // The default policy until the first navigation; a page a script
        // opens and fills itself (window.open("")) never has one.
        applyAutoplay("about:blank")
        installPictureInPicture()
        installMediaSession()
    }

    // Sites' own picture-in-picture buttons reach Ion through this script,
    // which logs one of two fixed messages (see ion_media::pip).
    function installPictureInPicture() {
        const script = WebEngine.script()
        script.name = "ion-pip"
        script.sourceCode = Media.pipPageScript
        script.injectionPoint = WebEngineScript.DocumentCreation
        script.worldId = WebEngineScript.MainWorld
        script.runsOnSubFrames = true
        userScripts.insert(script)
    }
    // Pages report what they play through this script (ion_media::session);
    // Ion's now-playing controls and the system's answer with mediaAction.
    function installMediaSession() {
        const script = WebEngine.script()
        script.name = "ion-media-session"
        script.sourceCode = Media.sessionScript
        script.injectionPoint = WebEngineScript.DocumentCreation
        script.worldId = WebEngineScript.MainWorld
        script.runsOnSubFrames = true
        userScripts.insert(script)
    }
    function mediaReported(report) {
        const frames = Object.assign({}, mediaFrames)
        if (report.state)
            frames[report.frame] = report.state
        else
            delete frames[report.frame]
        mediaFrames = frames
        let best = null
        let bestFrame = ""
        for (const id in frames) {
            if (!best || (frames[id].playing && !best.playing)) {
                best = frames[id]
                bestFrame = id
            }
        }
        mediaFrame = bestFrame
        media = best
    }
    function clearMedia() {
        mediaFrames = {}
        mediaFrame = ""
        media = null
    }
    onLifecycleStateChanged: if (lifecycleState === WebEngineView.LifecycleState.Discarded) clearMedia()
    // `action` as for Media.actionScript ("playpause", "next", "seekby", …).
    function mediaAction(action, value) {
        const script = Media.actionScript(action, value || 0, mediaFrame)
        if (script.length > 0)
            runInFrames(mainFrame, script)
    }
    onJavaScriptConsoleMessage: (level, message, lineNumber, sourceId) => {
        if (message === Media.pipEnterMessage)
            pictureInPictureRequested(true)
        else if (message === Media.pipExitMessage)
            pictureInPictureRequested(false)
        else {
            const report = Media.sessionReport(message)
            if (report)
                mediaReported(report)
        }
    }

    // A site of its own may darken differently.
    onUrlChanged: {
        live = true
        const darken = Theme.engine.darkenPage(url.toString())
        if (settings.forceDarkMode !== darken)
            settings.forceDarkMode = darken
        loadError.item?.leftFor(url)
        // A page's own `history.pushState`/`replaceState` moves the tab to a
        // new entry without a load; the next redirect chain starts there.
        Qt.callLater(() => {
            const committed = view.currentHistoryUrl()
            if (committed !== "" && committed === view.url.toString()) {
                view.navigationStart = committed
                view.navigationEnd = committed
            }
        })
    }

    // Creating a tab resets the scheme QtWebEngine hands to pages, so restore
    // Ion's when this tab's first load starts. Later loads don't reset it.
    property bool pageSchemeRestored: false
    onLoadingChanged: info => {
        live = true
        autoplayLoading(info)
        if (info.status === WebEngineView.LoadSucceededStatus)
            snapshotSoon.restart()
        else if (info.status === WebEngineView.LoadStartedStatus && !visible)
            snapshot = ""
        // Ion's own error page covers Chromium's until something replaces it;
        // a load that only starts, or is stopped, leaves Chromium's in place.
        // HTTPS-first: the https page Ion asked for failed; go back to http
        // ("upgrade") or offer it ("strict").
        // Only a connection or certificate failure means "no https here"; a
        // load the person stopped doesn't.
        const noHttps = info.status === WebEngineView.LoadFailedStatus
                && (info.errorDomain === WebEngineView.ConnectionErrorDomain
                    || info.errorDomain === WebEngineView.CertificateErrorDomain)
                && info.errorCode !== -3
        const insecure = noHttps ? Privacy.httpsFallback(info.url.toString()) : ""
        if (insecure.length > 0 && Privacy.httpsMode() === "strict") {
            loadError.item.showInsecureOffer(info, insecure)
        } else if (insecure.length > 0) {
            Qt.callLater(() => view.url = insecure)
            notify(qsTr("%1 has no secure connection; loaded without HTTPS")
                   .arg(insecure.replace(/^http:\/\//, "").split("/")[0]))
        } else if (info.status === WebEngineView.LoadFailedStatus)
            loadError.item.showFor(info)
        else if (info.status === WebEngineView.LoadSucceededStatus && loadError.item)
            loadError.item.visible = false
        if (!pageSchemeRestored && info.status === WebEngineView.LoadStartedStatus) {
            pageSchemeRestored = true
            Theme.engine.applyPageScheme()
            applyPageTheme()
        }
        // A new document starts with nothing playing; the old one's frames
        // never say goodbye.
        if (info.status === WebEngineView.LoadStartedStatus)
            clearMedia()
        // Failed loads too, so an error page doesn't keep the last site's zoom.
        if (info.status === WebEngineView.LoadSucceededStatus
                || info.status === WebEngineView.LoadFailedStatus)
            zoomFactor = Zoom.factorFor(url)
        // Once a load ends (or is stopped), the redirect chain is over and
        // the tab is on whatever entry committed.
        if (info.status !== WebEngineView.LoadStartedStatus) {
            reloading = false
            const committed = currentHistoryUrl()
            if (committed !== "") {
                navigationStart = committed
                navigationEnd = committed
            }
        }
    }
    onNewWindowRequested: request => view.newTabRequested(request)

    // Ion's login-form helper talks to its page script over this channel,
    // in the isolated world the page's own scripts can't see.
    webChannel: formFill.channel
    webChannelWorld: WebEngineScript.ApplicationWorld
    // A crashed (or killed) page shows Ion's page instead of a blank tab;
    // Try again or Reload starts a fresh page process.
    onRenderProcessTerminated: (terminationStatus, exitCode) => {
        clearMedia()
        if (terminationStatus !== WebEngineView.NormalTerminationStatus)
            loadError.item?.showCrash(terminationStatus, view.url)
    }

    // Both are off by default; Ion asks before a page uses them.
    settings.fullScreenSupportEnabled: true
    settings.screenCaptureEnabled: true
    // Copy buttons and editors (Google Docs, Notion, GitHub) use the async
    // clipboard. Writing needs nothing more; reading asks first (Safety's
    // clipboard permission).
    settings.javascriptCanAccessClipboard: true
    settings.javascriptCanPaste: true
    // PDFs open in Chromium's viewer (zoom, page thumbnails, search,
    // print, download) instead of only downloading. The viewer is built in
    // as a plugin, so plugins have to be on for it; Qt 6 runs no others.
    settings.pluginsEnabled: true
    settings.pdfViewerEnabled: true

    // Zoom, remembered per site by the `Zoom` singleton.
    function setZoom(factor) {
        zoomFactor = factor
        Zoom.remember(url, factor)
    }
    function zoomIn() { setZoom(Zoom.stepIn(zoomFactor)) }
    function zoomOut() { setZoom(Zoom.stepOut(zoomFactor)) }
    function resetZoom() { setZoom(1.0) }

    // A small picture of the page for the tab strips' hover previews, taken
    // while the tab is on screen: after each load and every so often. Kept as
    // the grab result, which owns the image `snapshot` points to.
    // None at all while previews are off (ui.tabPreviews).
    property url snapshot: ""
    property var snapshotResult: null
    readonly property bool snapshots: { Config.revision; return Config.value("ui.tabPreviews") !== false }
    onSnapshotsChanged: if (!snapshots) { snapshot = ""; snapshotResult = null }
    function takeSnapshot() {
        if (!snapshots || !visible || width <= 0 || height <= 0 || loadError.item?.visible)
            return
        grabToImage(result => {
            snapshotResult = result
            snapshot = result.url
        }, Qt.size(Theme.previewWidth * Screen.devicePixelRatio,
                   Math.round(Theme.previewWidth * Screen.devicePixelRatio * height / width)))
    }
    Timer {
        id: snapshotSoon
        interval: Theme.noticeMs / 2
        onTriggered: view.takeSnapshot()
    }
    Timer {
        interval: Theme.snapshotMs
        repeat: true
        running: view.snapshots && view.visible && Qt.application.state === Qt.ApplicationActive
        onTriggered: view.takeSnapshot()
    }
    // Saves the visible area (or the whole page) as a PNG in the downloads
    // folder and copies it to the clipboard.
    function takeScreenshot(fullPage) { screenshotTool.item?.capture(fullPage) }

    // The new-tab page exists only while it can show: a blank tab on screen.
    // Built per tab, it would hold a dozen tiles in every tab and rebuild
    // them all on each config change.
    TabPart {
        active: view.live && view.visible && view.showNewTabPage && Basics.isNewTabUrl(view.url)
        sourceComponent: NewTabPage { view: browserTab }
    }
    // Eager: the tab's webChannel is this part's.
    FormFill {
        id: formFill
        view: view
        onManageRequested: section => view.managePasswords(section)
    }
    BuiltinPage { id: builtinPage; view: view }
    TabPart { sourceComponent: AdblockCosmetics { view: browserTab } }
    FindBar { id: findBar; view: view }
    TabPart { sourceComponent: PermissionPrompt { view: browserTab; tabId: browserTab.tabId } }
    ExternalAppPrompt { id: externalApp; view: view }
    TabPart { sourceComponent: ScreenSharePicker { view: browserTab } }
    TabPart { id: pageDialog; sourceComponent: PageDialog { view: browserTab } }
    WebAuthDialog { view: view }
    TabPart {
        id: screenshotTool
        sourceComponent: ScreenshotTool { view: browserTab }
    }
    TabPart {
        sourceComponent: ContextMenuHandler {
            view: browserTab
            omnibox: tabOmnibox
            onOpenInNewTab: target => view.openInNewTab(target)
            onAskAgent: input => view.askAgent(input)
        }
    }
    Omnibox {
        id: tabOmnibox
        searchEngineName: Config.searchEngineName
        searchTemplate: Config.searchTemplate
    }
    // Show a short message over the page.
    function notify(message) {
        notice.show(message)
    }

    ReaderMode { id: readerMode; view: view }
    ReaderBar { reader: readerMode }
    PageTranslator { id: translator; view: view }
    PageNotice { id: notice }

    // Per-site JavaScript switch from `[sites]` in config, applied as each
    // page starts loading.
    // Where the current main-frame navigation started; its user agent is
    // the one the whole redirect chain is sent with.
    property string navigationStart: ""
    // Where that chain has got to, after any server redirects.
    property string navigationEnd: ""
    // Whether that navigation is to the page the tab is already on (a
    // reload, or a link to the same URL) and hasn't finished loading.
    property bool reloading: false

    // The URL of the history entry the tab is on, or "" before the first
    // page commits.
    function currentHistoryUrl() {
        const items = view.history.items
        const current = view.history.backItems.rowCount()
        if (current < 0 || current >= items.rowCount())
            return ""
        // Role 256 (Qt::UserRole) is WebEngineHistoryModel's UrlRole.
        return String(items.data(items.index(current, 0), 256))
    }

    onNavigationRequested: request => {
        // Links to other apps (mailto:, zoommtg:…) ask first, from frames too.
        if (Protocols.isExternal(request.url.toString())) {
            request.reject()
            externalApp.ask(request.url.toString(), view.url.toString())
            return
        }
        if (!request.isMainFrame)
            return
        const target = request.url.toString()
        // A page script's `location.replace` also counts as a redirect, but
        // the chain it replaces has already committed (unless that chain went
        // to the page the tab was on, whose entry matches before it commits).
        // It is a fresh request with its own header, so it starts a chain.
        const redirect = request.navigationType === WebEngineNavigationRequest.RedirectNavigation
        const fromPage = redirect && !view.reloading
                && view.currentHistoryUrl() === view.navigationEnd
        // Chromium ignores a User-Agent change on a server redirect, so one
        // into a site with another `userAgent` would arrive with the wrong
        // one. Start it over as a fresh navigation, which gets the header;
        // not when it carries a form (a 307/308 after a POST), since a
        // fresh navigation would turn it into a GET and drop the form.
        if (redirect && !fromPage && !request.hasFormData
                && !Sites.sameUserAgent(view.navigationStart, target)) {
            request.reject()
            Qt.callLater(() => view.url = target)
            return
        }
        if (!redirect || fromPage) {
            view.navigationStart = target
            view.reloading = request.navigationType === WebEngineNavigationRequest.ReloadNavigation
                    || target === view.currentHistoryUrl()
        }
        view.navigationEnd = target
        view.settings.javascriptEnabled = Sites.javascriptEnabled(target)
        view.applyAutoplay(target)
        autoplayCheck.restart()
    }

    // A navigation can still be called off after it's requested (a
    // beforeunload prompt, a download): the page that stays keeps its own
    // autoplay policy. A load that starts takes its destination's; one that
    // ends, or never starts, goes back to the committed page's.
    Timer {
        id: autoplayCheck
        interval: 1500
        onTriggered: view.applyAutoplay(view.autoplayUrl())
    }
    // A blank popup's inherited site lasts until a real page starts loading,
    // so a navigation that's called off leaves it in place. A settings change
    // that came during the load applies to the page once it's done.
    function autoplayLoading(info) {
        if (info.status === WebEngineView.LoadStartedStatus) {
            autoplayCheck.stop()
            const target = info.url.toString()
            if (!target.startsWith("about:"))
                autoplayInherited = ""
            applyAutoplay(target)
        } else if (autoplaySettingsChanged) {
            autoplaySettingsChanged = false
            reconcileAutoplay()
        } else {
            applyAutoplay(autoplayUrl())
        }
    }

    // A blank popup's opener, whose site's policy the popup keeps while it
    // stays blank (set by Main.qml when it adopts the popup).
    property string autoplayInherited: ""
    // The URL whose policy the open page follows.
    function autoplayUrl() {
        const current = url.toString()
        return autoplayInherited.length > 0 && (current.length === 0 || current.startsWith("about:"))
            ? autoplayInherited : current
    }

    // The policy last applied, to spot a settings change that affects the
    // open page.
    property string autoplayPolicy: ""

    // `[media] autoplay` or a `[sites]` entry changed: the open page follows
    // at once. "block" is enforced in its frames now; the engine's gesture
    // rule for sound takes effect from the page's next load.
    Connections {
        target: Config
        function onRevisionChanged() {
            if (view.loading)
                view.autoplaySettingsChanged = true
            else
                view.reconcileAutoplay()
        }
    }
    property bool autoplaySettingsChanged: false
    function reconcileAutoplay() {
        const before = autoplayPolicy
        applyAutoplay(autoplayUrl())
        if (autoplayPolicy === before)
            return
        if (autoplayPolicy === "block")
            runInFrames(mainFrame, Media.blockAutoplayScript, WebEngineScript.MainWorld)
        else if (before === "block")
            runInFrames(mainFrame, Media.unblockAutoplayScript, WebEngineScript.MainWorld)
    }

    // `[media] autoplay` for the page about to load. The engine's gesture
    // rule lets silent media play and holds sound back; "block" adds a page
    // script that holds silent media back too.
    function applyAutoplay(target) {
        const policy = Media.autoplay(target)
        autoplayPolicy = policy
        settings.playbackRequiresUserGesture = policy !== "allow"
        const installed = userScripts.find("ion-autoplay-block")
        if (policy === "block" && installed.length === 0) {
            const script = WebEngine.script()
            script.name = "ion-autoplay-block"
            script.sourceCode = Media.blockAutoplayScript
            script.injectionPoint = WebEngineScript.DocumentCreation
            script.worldId = WebEngineScript.MainWorld
            script.runsOnSubFrames = true
            userScripts.insert(script)
        } else if (policy !== "block" && installed.length > 0) {
            userScripts.remove(installed[0])
        }
    }

    Connections {
        target: Theme.engine
        function onPageSchemeChanged() {
            view.applyPageTheme()
        }
    }

    // Failed loads show LoadErrorPage, in Ion's colors, over Chromium's error
    // page. That page stays enabled underneath because it makes the failed
    // address the current entry, so the URL bar, title, Reload and Back treat
    // it like any other page.
    TabPart {
        id: loadError
        sourceComponent: LoadErrorPage {
            anchors.fill: parent
            onRetry: browserTab.reload()
            onContinueInsecure: target => {
                Privacy.allowHttp(target)
                browserTab.url = target
            }
        }
    }
}
