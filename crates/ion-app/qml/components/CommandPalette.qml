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
// focusUrlBar() and currentView.
Popup {
    id: root

    required property var browser
    property var results: []
    // Global pointer position last seen over a row; see the rows' MouseArea.
    property point lastPointer

    readonly property int rowHeight: Theme.tabHeight + Theme.spacing * 2
    readonly property bool mac: Qt.platform.os === "osx" || Qt.platform.os === "macos"

    // Open with `text` already typed, e.g. ">" for commands only.
    function show(text) {
        field.text = text ?? ""
        open()
        // Now rather than in onOpened, after the enter transition: the first
        // keys typed right after Ctrl/Cmd+K must not be lost.
        field.forceActiveFocus()
    }

    function refresh() {
        const titles = [], urls = []
        for (let i = 0; i < Tabs.count; ++i) {
            titles.push(Tabs.titleAt(i))
            urls.push(Tabs.urlAt(i))
        }
        // `>` and `!` narrow the palette to commands or bangs; skip history then.
        const text = field.text.trim()
        const history = text.length > 0 && !text.startsWith(">") && !text.startsWith("!")
            ? History.search(text, Theme.paletteMaxRows) : "[]"
        results = search.query(field.text, titles, urls, Tabs.currentIndex, history, Tabs.sessionNames())
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
        case "focus-url": browser.focusUrlBar(); break
        case "reload": view?.reload(); break
        case "back": view?.goBack(); break
        case "forward": view?.goForward(); break
        case "quit": Qt.quit(); break
        default: console.warn("CommandPalette: unknown command", id)
        }
    }

    function glyph(kind) {
        switch (kind) {
        case "tab": return "▭"
        case "history": return "↺"
        case "session": return "▤"
        case "setting": return "⚙"
        case "command": return "›"
        case "bang": return "!"
        case "open": return "↗"
        default: return "⌕"
        }
    }

    // Shortcut hints are written as "Ctrl+…"; show them the macOS way there.
    function hintText(hint) {
        if (!mac)
            return hint
        return hint.replace("Alt+Left", "Ctrl+[").replace("Alt+Right", "Ctrl+]")
            .replace("Ctrl+", "⌘").replace("Shift+", "⇧")
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
            placeholderText: qsTr("Search tabs, history, commands, settings and !bangs")
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

            delegate: Rectangle {
                id: row

                required property var modelData
                required property int index

                width: ListView.view.width
                height: root.rowHeight
                radius: Theme.radius
                // One highlight only: pointing at a row selects it, like the arrow keys.
                color: ListView.isCurrentItem ? Theme.surfaceRaised : "transparent"

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.spacing * 2
                    anchors.rightMargin: Theme.spacing * 2
                    spacing: Theme.spacing * 2

                    Text {
                        Layout.preferredWidth: Theme.iconSize
                        text: root.glyph(row.modelData.kind)
                        color: row.ListView.isCurrentItem ? Theme.accent : Theme.textMuted
                        font.pixelSize: Theme.fontSize + 2
                        horizontalAlignment: Text.AlignHCenter
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0

                        Text {
                            Layout.fillWidth: true
                            text: row.modelData.title
                            color: Theme.text
                            font.pixelSize: Theme.fontSize
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            visible: text.length > 0
                            text: row.modelData.subtitle
                            color: Theme.textMuted
                            font.pixelSize: Theme.fontSize - 2
                            elide: Text.ElideMiddle
                        }
                    }

                    Text {
                        visible: text.length > 0
                        text: root.hintText(row.modelData.hint)
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSize - 1
                    }
                }

                MouseArea {
                    id: rowMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    // Moving the pointer selects. Rows sliding under a resting
                    // pointer (the palette opening, results changing) do not.
                    onPositionChanged: mouse => {
                        const p = mapToGlobal(mouse.x, mouse.y)
                        const last = root.lastPointer
                        root.lastPointer = p
                        if (last.x >= 0 && (last.x !== p.x || last.y !== p.y))
                            list.currentIndex = row.index
                    }
                    onClicked: mouse => root.choose(row.modelData, mouse.modifiers & Qt.ControlModifier)
                }
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
