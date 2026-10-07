import QtQuick
import QtQuick.Controls
import Ion

// The address field. Shows the current page URL; while typing, a list below it
// suggests what Enter will do, other open tabs, history pages and bang
// completions (ranked by `PaletteSearch.suggest` in Rust). Enter takes the
// highlighted suggestion; Alt+Enter opens it in a new tab.
TextField {
    id: field

    // URL of the page in the current tab.
    property url currentUrl
    // Emitted with the resolved URL when the person presses Enter.
    signal navigate(url target)
    // Emitted after Enter or Escape, so the page can take keyboard focus back.
    signal finished()

    property var suggestions: []
    // What the person typed, without any address filled in after it.
    property string typed: ""
    // Global pointer position last seen over a suggestion.
    property point lastPointer: Qt.point(-1, -1)

    Omnibox {
        id: omnibox
        searchEngineName: Config.searchEngineName
        searchTemplate: Config.searchTemplate
    }
    PaletteSearch { id: search }

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

    function showUrl() {
        text = currentUrl.toString() === "about:blank" ? "" : currentUrl.toString()
    }

    // Fills in the rest of an address when typing added to the end of the
    // text; the filled part is selected, so the next key replaces it.
    function edited() {
        const grew = text.length > typed.length && cursorPosition === text.length
        typed = text
        refreshSuggestions(grew)
    }

    function refreshSuggestions(fill) {
        const query = text.trim()
        if (!activeFocus || query.length === 0) {
            suggestions = []
            return
        }
        const titles = [], urls = []
        for (let i = 0; i < Tabs.count; ++i) {
            titles.push(Tabs.titleAt(i))
            urls.push(Tabs.urlAt(i))
        }
        const history = query.startsWith("!") ? "[]" : History.search(query, Theme.paletteMaxRows)
        if (fill) {
            const filled = search.autocomplete(text, urls, history)
            if (filled.length > text.length) {
                text = filled
                select(typed.length, filled.length)
            }
        }
        suggestions = search.suggest(text, titles, urls, Tabs.currentIndex, history)
        list.currentIndex = 0
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

    function choose(item, inNewTab) {
        switch (item.action) {
        case "complete":
            text = item.value
            typed = text
            refreshSuggestions(false)
            break
        case "tab":
            done()
            Tabs.activate(item.value)
            break
        case "open":
            done()
            openUrl(item.value, inNewTab)
            break
        }
    }

    function openUrl(target, inNewTab) {
        if (inNewTab)
            Tabs.openTab(target.toString(), true)
        else
            navigate(target)
    }

    onCurrentUrlChanged: if (!activeFocus) showUrl()
    onActiveFocusChanged: {
        if (activeFocus) {
            typed = ""
            selectAll()
        } else {
            suggestions = []
            showUrl()
        }
    }
    onTextEdited: edited()

    Keys.onReturnPressed: event => go(event.modifiers & Qt.AltModifier)
    Keys.onEnterPressed: event => go(event.modifiers & Qt.AltModifier)
    Keys.onUpPressed: list.decrementCurrentIndex()
    Keys.onDownPressed: list.incrementCurrentIndex()
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

        background: Rectangle {
            radius: Theme.radius
            color: Theme.surface
            border.width: 1
            border.color: Theme.border
        }

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
