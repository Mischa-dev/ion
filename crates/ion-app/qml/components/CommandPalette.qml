import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Ctrl/Cmd+K palette: one search over open tabs, history, saved sessions, Ion
// commands, settings and bangs. Ranking lives in Rust (`PaletteSearch`); this file
// gathers the sources, shows the results and carries out the chosen one.
//
// Tabs and history come from the `Tabs` and `History` singletons, bookmarks
// from `Bookmarks`. `browser` (the main window) provides openTab(url),
// runCommand(id) and canRunCommand(id) (commands live in Commands.qml) and
// currentView.
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
        const found = search.query(field.text, titles, urls, Tabs.currentIndex, history, bookmarks, Tabs.sessionNames())
        // Only commands that can run now (agent ones need agents turned on).
        results = found.filter(item => item.action !== "run" || browser.canRunCommand(item.value))
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
            Keymap.noteUsed(item.value)
            browser.runCommand(item.value)
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

    PaletteSearch { id: search }

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
        color: Theme.scrim
    }

    background: IonSurface {}

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
        NumberAnimation { property: "scale"; from: Theme.popupStartScale; to: 1; duration: Theme.animationSlowMs; easing.type: Theme.easeOut }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
    }

    contentItem: ColumnLayout {
        spacing: Theme.spacing
        // Rows scrolled out of the list never draw below the card.
        clip: true

        TextField {
            id: field
            Layout.fillWidth: true
            color: Theme.text
            placeholderText: qsTr("Search tabs, bookmarks, history, commands, settings and !bangs")
            placeholderTextColor: Theme.textMuted
            selectionColor: Theme.accent
            selectedTextColor: Theme.onAccent
            leftPadding: Theme.spacing * 3 + Theme.iconSize
            rightPadding: Theme.spacing * 2
            verticalAlignment: TextInput.AlignVCenter
            selectByMouse: true
            implicitHeight: Theme.urlBarHeight + Theme.spacing * 2
            font.pixelSize: Theme.fontSizeLarge

            background: Rectangle {
                radius: Theme.radius
                color: "transparent"

                Icon {
                    x: Theme.spacing * 2
                    anchors.verticalCenter: parent.verticalCenter
                    name: "search"
                    color: Theme.textMuted
                }
                // Hairline between the query and the results.
                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: -Theme.spacing / 2
                    height: Theme.hairline
                    color: Theme.border
                }
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
                query: field.text.replace(/^[>*!]/, "")
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
