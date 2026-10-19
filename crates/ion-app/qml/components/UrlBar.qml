import QtQuick
import QtQuick.Controls
import Ion

// The address field. Unfocused it shows the page's address without the
// noise (no https:// or www.); focused, the full address. While typing, a
// list below it suggests what Enter will do, inline answers (sums, unit
// conversions), open tabs, bookmarks, history, quick actions and the search
// engine's suggestions, ranked by `AddressBar.suggest` in Rust. Enter takes
// the highlighted row; Alt+Enter opens it in a new tab; Shift+Delete removes
// a highlighted history page. With the field emptied it lists top sites.
TextField {
    id: field

    // URL of the page in the current tab.
    property url currentUrl
    // Emitted with the resolved URL when the person presses Enter.
    signal navigate(url target)
    // Emitted after Enter or Escape, so the page can take keyboard focus back.
    signal finished()
    // Emitted with the typed text when the person asks an agent (`@ion …`,
    // `!ai …`).
    signal askAgent(string input)
    // Emitted with a command id from `ion_bangs::commands` for quick actions.
    signal runCommand(string id)

    property var suggestions: []
    // What the person typed, without any address filled in after it.
    property string typed: ""
    // Global pointer position last seen over a suggestion.
    property point lastPointer: Qt.point(-1, -1)
    // The search engine's suggestions and the query they are for.
    property var remote: []
    property string remoteQuery: ""
    // Recent engine answers by query, so backspacing doesn't refetch. No
    // prototype, so queries like "constructor" aren't already "cached".
    property var remoteCache: Object.create(null)
    property var request: null

    readonly property bool suggestionsWanted: {
        Config.revision
        return Config.value("search.suggestions") !== false
    }

    Omnibox {
        id: omnibox
        searchEngineName: Config.searchEngineName
        searchTemplate: Config.searchTemplate
    }
    // Changing the search engine makes cached suggestions stale.
    Connections {
        target: Config
        function onRevisionChanged() {
            if (field.request) {
                field.request.abort()
                field.request = null
            }
            field.remoteCache = Object.create(null)
            field.remote = []
            field.remoteQuery = ""
            // Drop rows for the old engine now, then ask the new one.
            if (field.activeFocus) {
                field.refreshSuggestions(false, true)
                field.fetchRemote()
            }
        }
    }
    // QML has no clipboard API; an invisible text field copies answers.
    TextInput { id: clipboard; visible: false }

    implicitHeight: Theme.urlBarHeight
    color: Theme.text
    placeholderText: qsTr("Search or enter address")
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
        border.width: field.activeFocus ? 1 : 0
        border.color: Theme.accent
    }

    // The address for reading: "github.com/Mischa-dev/ion" rather than
    // "https://www.github.com/Mischa-dev/ion". Plain http:// stays visible.
    function displayUrl(url) {
        let text = url.replace(/^https:\/\//, "")
        if (text !== url)
            text = text.replace(/^www\./, "")
        // A bare site drops its trailing slash; paths keep theirs.
        if (/^[^/]+\/$/.test(text))
            text = text.slice(0, -1)
        return text.length > 0 ? text : url
    }

    function showUrl() {
        const url = currentUrl.toString()
        if (url === "about:blank")
            text = ""
        else
            text = activeFocus ? url : displayUrl(url)
        cursorPosition = 0
    }

    // Fills in the rest of an address when typing added to the end of the
    // text; the filled part is selected, so the next key replaces it.
    function edited() {
        const grew = text.length > typed.length && cursorPosition === text.length
        typed = text
        refreshSuggestions(grew)
        fetchRemote()
    }

    function tabLists() {
        const titles = [], urls = []
        for (let i = 0; i < Tabs.count; ++i) {
            titles.push(Tabs.titleAt(i))
            urls.push(Tabs.urlAt(i))
        }
        return { titles: titles, urls: urls }
    }

    function refreshSuggestions(fill, keepSelection) {
        if (!activeFocus) {
            suggestions = []
            return
        }
        // Until the person types, the field holds the page's address; the
        // list then shows top sites rather than suggestions for that address.
        let input = typed.length > 0 || text.length === 0 ? text : ""
        // Pages and the engine's suggestions are for what was typed, not
        // for an address filled in after it.
        const query = (input.length > 0 && typed.length > 0 ? typed : input).trim()
        // Addressed to an agent: that is the only thing Enter can mean.
        const agentRows = query.length > 0 ? JSON.parse(Agent.suggest(input)) : []
        if (agentRows.length > 0) {
            suggestions = agentRows
            list.currentIndex = 0
            return
        }
        const tabs = tabLists()
        const plain = !query.startsWith("!")
        const history = plain ? History.search(query, Theme.paletteMaxRows * 2) : "[]"
        const chosen = plain && query.length > 0 ? History.chosen(query) : "[]"
        if (fill) {
            const filled = AddressBar.autocomplete(input, tabs.urls, history, chosen)
            if (filled.length > text.length) {
                text = filled
                select(typed.length, filled.length)
                input = text
            }
        }
        const bookmarks = plain && query.length > 0 ? Bookmarks.search(query, Theme.paletteMaxRows) : "[]"
        const engine = remoteQuery === query ? remote : (remoteCache[query] ?? [])
        const previous = keepSelection ? suggestions[list.currentIndex] : null
        suggestions = AddressBar.suggest(input, typed, tabs.titles, tabs.urls, Tabs.currentIndex,
                                         history, bookmarks, chosen, engine)
        list.currentIndex = previous ? Math.max(0, suggestions.findIndex(
            s => s.kind === previous.kind && s.action === previous.action
                && s.value === previous.value && s.title === previous.title)) : 0
    }

    // Ask the search engine for suggestions, a beat after typing settles.
    function fetchRemote() {
        if (request) {
            request.abort()
            request = null
        }
        remoteTimer.stop()
        const query = typed.trim()
        if (!suggestionsWanted || query.length === 0 || remoteCache[query] !== undefined)
            return
        remoteTimer.restart()
    }

    Timer {
        id: remoteTimer
        interval: 60
        onTriggered: {
            const query = field.typed.trim()
            const url = AddressBar.suggestUrl(query)
            if (url.length === 0)
                return
            const xhr = new XMLHttpRequest()
            xhr.onreadystatechange = () => {
                if (xhr.readyState !== XMLHttpRequest.DONE || field.request !== xhr)
                    return
                field.request = null
                if (xhr.status !== 200)
                    return
                const found = AddressBar.parseSuggestions(xhr.responseText, query)
                const cache = field.remoteCache
                if (Object.keys(cache).length > 200)
                    field.remoteCache = Object.create(null)
                field.remoteCache[query] = found
                field.remote = found
                field.remoteQuery = query
                if (field.activeFocus && field.typed.trim() === query)
                    field.refreshSuggestions(false, true)
            }
            xhr.open("GET", url)
            field.request = xhr
            xhr.send()
        }
    }

    // Hand focus back to the page, which shows the current address again.
    function done() {
        suggestions = []
        finished()
    }

    function go(inNewTab) {
        const item = suggestions.length > 0 ? suggestions[list.currentIndex] : null
        if (item) {
            choose(item, inNewTab)
            return
        }
        const target = omnibox.resolve(text)
        if (target.toString().length === 0)
            return
        done()
        openUrl(target, inNewTab)
    }

    // Teach the address bar which page this text led to.
    // Learns which page these words lead to; private windows learn nothing.
    function remember(url, title) {
        if (Privacy.windowKind() === "private")
            return
        if (typed.trim().length > 0 && url.length > 0)
            History.recordChoice(typed, url, title)
    }

    function choose(item, inNewTab) {
        switch (item.action) {
        case "complete":
            text = item.value
            typed = text
            refreshSuggestions(false)
            fetchRemote()
            break
        case "tab":
            remember(Tabs.urlAt(item.value), Tabs.titleAt(item.value))
            done()
            Tabs.activate(item.value)
            break
        case "open":
            if (item.kind === "tab" || item.kind === "bookmark" || item.kind === "history")
                remember(item.value, item.title)
            else if (item.kind === "open")
                remember(item.value, "")
            done()
            openUrl(item.value, inNewTab)
            break
        case "copy":
            clipboard.text = item.value
            clipboard.selectAll()
            clipboard.copy()
            done()
            break
        case "run":
            done()
            runCommand(item.value)
            break
        case "agent":
            done()
            askAgent(item.value)
            break
        }
    }

    function openUrl(target, inNewTab) {
        if (inNewTab)
            Tabs.openTab(target.toString(), true)
        else
            navigate(target)
    }

    // Shift+Delete forgets the highlighted history page.
    function removeCurrent() {
        const item = suggestions[list.currentIndex]
        if (!item || item.kind !== "history" || item.action !== "open")
            return false
        History.remove(item.value)
        History.save()
        refreshSuggestions(false, false)
        return true
    }

    onCurrentUrlChanged: if (!activeFocus) showUrl()
    onActiveFocusChanged: {
        if (activeFocus) {
            typed = ""
            remote = []
            remoteQuery = ""
            const url = currentUrl.toString()
            text = url === "about:blank" ? "" : url
            selectAll()
        } else {
            remoteTimer.stop()
            if (request) {
                request.abort()
                request = null
            }
            suggestions = []
            showUrl()
        }
    }
    onTextEdited: edited()

    Keys.onReturnPressed: event => go(event.modifiers & Qt.AltModifier)
    Keys.onEnterPressed: event => go(event.modifiers & Qt.AltModifier)
    Keys.onUpPressed: list.decrementCurrentIndex()
    Keys.onDownPressed: {
        // Down in an empty or untouched field opens the list (top sites).
        if (suggestions.length === 0) {
            refreshSuggestions(false)
            return
        }
        list.incrementCurrentIndex()
    }
    Keys.onDeletePressed: event => {
        if ((event.modifiers & Qt.ShiftModifier) && removeCurrent())
            return
        event.accepted = false
    }
    // Tab completes the highlighted bang, or switches to the site search
    // offered for a typed trigger ("gh" → "!gh ").
    Keys.onTabPressed: event => {
        const selected = suggestions[list.currentIndex]
        const item = selected && selected.action === "complete" ? selected
            : suggestions.find(s => s.action === "complete" && s.hint === "Tab")
        if (item)
            choose(item, false)
        else
            event.accepted = false
    }
    Keys.onEscapePressed: {
        if (suggestions.length > 0) {
            // Dismissed: a late engine answer mustn't reopen the list.
            remoteTimer.stop()
            if (request) {
                request.abort()
                request = null
            }
            suggestions = []
            return
        }
        showUrl()
        finished()
    }

    Popup {
        id: popup

        y: field.height + Theme.spacing / 2
        width: field.width
        padding: Theme.spacing / 2
        visible: field.suggestions.length > 0
        onAboutToShow: field.lastPointer = Qt.point(-1, -1)
        closePolicy: Popup.NoAutoClose

        background: IonSurface {}

        contentItem: ListView {
            id: list
            implicitHeight: contentHeight
            interactive: false
            highlightMoveDuration: 0
            model: field.suggestions

            delegate: CommandPaletteRow {
                required property var modelData
                required property int index

                width: ListView.view.width
                item: modelData
                query: field.typed
                current: ListView.isCurrentItem
                onPointerMoved: position => {
                    const last = field.lastPointer
                    field.lastPointer = position
                    if (last.x >= 0 && (last.x !== position.x || last.y !== position.y))
                        list.currentIndex = index
                }
                onChosen: modifiers => field.choose(modelData, modifiers & (Qt.ControlModifier | Qt.AltModifier))
            }
        }
    }
}
