import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Ctrl/Cmd+K palette: one search over open tabs, history, saved sessions, Ion
// commands, settings and bangs. Ranking lives in Rust (`PaletteSearch`); this file
// gathers the sources, shows the results and carries out the chosen one.
//
// Tabs and history come from the `Tabs` and `History` singletons; `browser`
// (the main window) provides openTab(url), newTab(), closeTab(i),
// focusUrlBar(), toggleBookmark(), showImport(), showExtensions(), profile
// and currentView. Bookmarks come from the `Bookmarks`
// singleton.
Popup {
    id: root

    required property var browser
    property var results: []
    // Global pointer position last seen over a row; see pointed().
    property point lastPointer

    readonly property int rowHeight: Theme.tabHeight + Theme.spacing * 2

    // Open with `text` already typed, e.g. ">" for commands only.
    function show(text) {
        field.text = text ?? ""
        open()
        // Now rather than in onOpened, after the enter transition: the first
        // keys typed right after Ctrl/Cmd+K must not be lost.
        field.forceActiveFocus()
    }

    function copyText(text) {
        clipboard.text = text
        clipboard.selectAll()
        clipboard.copy()
    }

    // Moving the pointer selects the row under it; a resting pointer doesn't.
    function pointed(index, position) {
        const last = lastPointer
        lastPointer = position
        if (last.x >= 0 && (last.x !== position.x || last.y !== position.y))
            list.currentIndex = index
    }

    function refresh() {
        const titles = [], urls = []
        for (let i = 0; i < Tabs.count; ++i) {
            titles.push(Tabs.titleAt(i))
            urls.push(Tabs.urlAt(i))
        }
        // `>` and `!` narrow the palette to commands or bangs; skip history and
        // bookmarks then. `*` lists bookmarks only.
        const text = field.text.trim()
        const pages = text.length > 0 && !text.startsWith(">") && !text.startsWith("!")
        const history = pages && !text.startsWith("*") ? History.search(text, Theme.paletteMaxRows) : "[]"
        const bookmarks = text.startsWith("*") ? Bookmarks.search(text.slice(1), 200)
            : pages ? Bookmarks.search(text, Theme.paletteMaxRows) : "[]"
        results = search.query(field.text, titles, urls, Tabs.currentIndex, history, bookmarks, Tabs.sessionNames())
        list.currentIndex = results.length > 0 ? 0 : -1
    }

    function choose(item, inNewTab) {
        if (!item)
            return
        if (item.action === "complete") {
            field.text = item.value
            return
        }
        close()
        // After the popup has handed focus back, so focusUrlBar() sticks.
        Qt.callLater(() => perform(item, inNewTab))
    }

    function perform(item, inNewTab) {
        const view = browser.currentView
        switch (item.action) {
        case "tab":
            Tabs.activate(item.value)
            break
        case "session":
            Tabs.openSession(item.value)
            break
        case "save-session":
            if (!Tabs.saveSessionAs(item.value))
                console.warn("CommandPalette: could not save session", item.value)
            break
        case "open":
            if (inNewTab || !view)
                browser.openTab(item.value)
            else
                view.url = item.value
            break
        case "run":
            run(item.value)
            break
        case "set":
            for (let i = 0; i + 1 < item.value.length; i += 2) {
                const error = Config.set(item.value[i], item.value[i + 1])
                if (error.length > 0)
                    console.warn("CommandPalette:", item.value[i], error)
            }
            break
        }
    }

    // Command ids come from `ion_bangs::commands::COMMANDS`.
    function run(id) {
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
        case "close-other-tabs": browser.closeOtherTabs(Tabs.currentIndex); break
        case "focus-url": browser.focusUrlBar(); break
        case "copy-url": copyText(view ? view.url.toString() : ""); break
        case "reload": view?.reload(); break
        case "hard-reload": view?.reloadAndBypassCache(); break
        case "stop": view?.stop(); break
        case "back": view?.goBack(); break
        case "forward": view?.goForward(); break
        case "bookmark-page": browser.toggleBookmark(); break
        case "bookmarks": Qt.callLater(() => root.show("*")); break
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
        case "fullscreen":
            browser.visibility = browser.visibility === Window.FullScreen ? Window.Windowed : Window.FullScreen
            break
        case "toggle-adblock":
            if (view && Adblock.setEnabledOn(view.url, !Adblock.isEnabledOn(view.url)))
                view.reload()
            break
        case "update-filter-lists": Adblock.updateLists(); break
        case "stop-agents": Safety.stopAgents(); break
        case "resume-agents": Safety.resumeAgents(); break
        case "screenshot": view?.takeScreenshot(false); break
        case "screenshot-page": view?.takeScreenshot(true); break
        case "quit": Qt.quit(); break
        default: console.warn("CommandPalette: unknown command", id)
        }
    }

    PaletteSearch { id: search }
    // QML has no clipboard API; an invisible text field copies for copyText().
    TextInput { id: clipboard; visible: false }

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Theme.urlBarHeight * 3
    width: Math.min(Theme.paletteWidth, parent.width - Theme.spacing * 4)
    padding: Theme.spacing
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    onAboutToShow: {
        lastPointer = Qt.point(-1, -1)
        refresh()
    }
    onOpened: field.forceActiveFocus()

    Overlay.modal: Rectangle {
        color: Qt.rgba(Theme.background.r, Theme.background.g, Theme.background.b, Theme.scrimOpacity)
    }

    background: Rectangle {
        radius: Theme.radius
        color: Theme.surface
        border.width: 1
        border.color: Theme.border
    }

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
        NumberAnimation { property: "scale"; from: 0.97; to: 1; duration: Theme.animationSlowMs; easing.type: Easing.OutCubic }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
    }

    contentItem: ColumnLayout {
        spacing: Theme.spacing

        TextField {
            id: field
            Layout.fillWidth: true
            implicitHeight: Theme.urlBarHeight
            color: Theme.text
            placeholderText: qsTr("Search tabs, bookmarks, history, commands, settings and !bangs")
            placeholderTextColor: Theme.textMuted
            selectionColor: Theme.accent
            selectedTextColor: Theme.onAccent
            font.pixelSize: Theme.fontSize
            leftPadding: Theme.spacing * 2
            rightPadding: Theme.spacing * 2
            verticalAlignment: TextInput.AlignVCenter
            selectByMouse: true

            background: Rectangle {
                radius: Theme.radius
                color: Theme.surfaceRaised
            }

            onTextChanged: if (root.opened || root.visible) root.refresh()

            Keys.onUpPressed: list.decrementCurrentIndex()
            Keys.onDownPressed: list.incrementCurrentIndex()
            Keys.onEscapePressed: root.close()
            Keys.onTabPressed: {
                const item = root.results[list.currentIndex]
                if (item && item.action === "complete")
                    field.text = item.value
            }
            // Enter runs the selection; Ctrl/Alt+Enter opens a page in a new tab.
            Keys.onReturnPressed: event => root.choose(root.results[list.currentIndex],
                                                          event.modifiers & (Qt.ControlModifier | Qt.AltModifier))
            Keys.onEnterPressed: event => root.choose(root.results[list.currentIndex],
                                                         event.modifiers & (Qt.ControlModifier | Qt.AltModifier))
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(count, Theme.paletteMaxRows) * root.rowHeight
            visible: count > 0
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            highlightMoveDuration: 0
            model: root.results

            delegate: CommandPaletteRow {
                required property var modelData
                required property int index

                width: ListView.view.width
                height: root.rowHeight
                item: modelData
                current: ListView.isCurrentItem
                onPointerMoved: position => root.pointed(index, position)
                onChosen: modifiers => root.choose(modelData, modifiers & Qt.ControlModifier)
            }
        }

        Text {
            Layout.fillWidth: true
            Layout.margins: Theme.spacing
            visible: list.count === 0
            text: qsTr("No matches")
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize
        }
    }
}
