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

    property var suggestions: []

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

    function refreshSuggestions() {
        const typed = text.trim()
        if (!activeFocus || typed.length === 0) {
            suggestions = []
            return
        }
        const titles = [], urls = []
        for (let i = 0; i < Tabs.count; ++i) {
            titles.push(Tabs.titleAt(i))
            urls.push(Tabs.urlAt(i))
        }
        const history = typed.startsWith("!") ? "[]" : History.search(typed, Theme.paletteMaxRows)
        suggestions = search.suggest(text, titles, urls, Tabs.currentIndex, history)
        list.currentIndex = 0
    }

    // Leave the field, showing the current page's address again.
    function done() {
        suggestions = []
        focus = false
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
            refreshSuggestions()
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
            selectAll()
        } else {
            suggestions = []
            showUrl()
        }
    }
    onTextEdited: refreshSuggestions()

    Keys.onReturnPressed: event => go(event.modifiers & Qt.AltModifier)
    Keys.onEnterPressed: event => go(event.modifiers & Qt.AltModifier)
    Keys.onUpPressed: list.decrementCurrentIndex()
    Keys.onDownPressed: list.incrementCurrentIndex()
    Keys.onTabPressed: event => {
        const item = suggestions[list.currentIndex]
        if (item && item.action === "complete")
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
        focus = false
    }

    Popup {
        id: popup

        y: field.height + Theme.spacing / 2
        width: field.width
        padding: Theme.spacing / 2
        visible: field.suggestions.length > 0
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

                width: ListView.view.width
                item: modelData
                current: ListView.isCurrentItem
                onChosen: modifiers => field.choose(modelData, modifiers & (Qt.ControlModifier | Qt.AltModifier))
            }
        }
    }
}
