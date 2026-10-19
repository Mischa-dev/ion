import QtQuick
import QtQuick.Layouts
import Ion

// One result row from `PaletteSearch` or `AddressBar`, shared by the command
// palette and the URL bar's suggestions: the site's icon (or a kind glyph),
// title and subtitle with the typed words in bold, and a right-aligned hint.
Rectangle {
    id: row

    // `{ kind, title, subtitle, hint, action, value }` from PaletteSearch.
    property var item: ({})
    // What was typed; its words are shown in bold.
    property string query: ""
    // Highlighted by the keyboard.
    property bool current: false
    // Clicked, with the keyboard modifiers held.
    signal chosen(int modifiers)
    // The pointer moved over the row; `position` is in global coordinates.
    // Lists select on real movement only, so rows sliding under a resting
    // pointer don't steal the keyboard selection.
    signal pointerMoved(point position)

    readonly property bool mac: Qt.platform.os === "osx" || Qt.platform.os === "macos"

    function glyph(kind) {
        switch (kind) {
        case "tab": return "▭"
        case "bookmark": return "★"
        case "history": return "↺"
        case "session": return "▤"
        case "setting": return "⚙"
        case "command": return "›"
        case "bang": return "!"
        case "open": return "↗"
        case "agent": return "@"
        case "answer": return "="
        default: return "⌕"
        }
    }

    // The page a row is about, for its favicon.
    readonly property string pageUrl: {
        const item = row.item
        if (item.action === "open" && item.kind !== "search" && item.kind !== "suggestion"
                && item.kind !== "bang")
            return item.value
        if (item.action === "tab")
            return Tabs.urlAt(item.value)
        return ""
    }

    function escaped(text) {
        return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    }

    // `text` as StyledText with the query's words in bold; `inverse` bolds
    // everything else instead, as for search suggestions, where what's new
    // is what matters.
    function marked(text, inverse) {
        const terms = row.query.toLowerCase().split(/\s+/).filter(t => t.length > 0)
        const lower = text.toLowerCase()
        if (terms.length === 0 || lower.length !== text.length)
            return escaped(text)
        const hit = new Array(text.length).fill(false)
        for (const term of terms) {
            for (let at = lower.indexOf(term); at >= 0; at = lower.indexOf(term, at + term.length)) {
                for (let k = 0; k < term.length; ++k)
                    hit[at + k] = true
            }
        }
        let html = "", bold = false
        for (let i = 0; i < text.length; ++i) {
            const want = hit[i] !== inverse
            if (want !== bold) {
                html += want ? "<b>" : "</b>"
                bold = want
            }
            html += escaped(text[i])
        }
        return bold ? html + "</b>" : html
    }

    // Shortcut hints are written as "Ctrl+…"; show them the macOS way there.
    function hintText(hint) {
        if (!mac)
            return hint
        return hint.replace("Alt+Left", "Ctrl+[").replace("Alt+Right", "Ctrl+]")
            .replace("Ctrl+", "⌘").replace("Shift+", "⇧")
    }

    implicitHeight: Theme.tabHeight + Theme.spacing * 2
    radius: Theme.radius
    // One highlight only: pointing at a row selects it, like the arrow keys.
    color: current ? Theme.surfaceRaised : "transparent"

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spacing * 2
        anchors.rightMargin: Theme.spacing * 2
        spacing: Theme.spacing * 2

        Item {
            Layout.preferredWidth: Theme.iconSize
            Layout.preferredHeight: Theme.iconSize

            Image {
                id: favicon
                anchors.fill: parent
                sourceSize: Qt.size(Theme.iconSize, Theme.iconSize)
                source: row.pageUrl.length > 0 ? "image://favicon/" + row.pageUrl : ""
                // Unknown icons load as an empty image.
                visible: status === Image.Ready && implicitWidth > 0
            }
            Text {
                anchors.centerIn: parent
                visible: !favicon.visible
                text: row.glyph(row.item.kind)
                color: row.current ? Theme.accent : Theme.textMuted
                font.pixelSize: Theme.fontSize + 2
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            Text {
                Layout.fillWidth: true
                text: row.marked(row.item.title ?? "", row.item.kind === "suggestion")
                textFormat: Text.StyledText
                color: Theme.text
                font.pixelSize: row.item.kind === "answer" ? Theme.fontSize + 2 : Theme.fontSize
                elide: Text.ElideRight
            }
            Text {
                Layout.fillWidth: true
                visible: (row.item.subtitle ?? "").length > 0
                text: row.marked(row.item.subtitle ?? "", false)
                textFormat: Text.StyledText
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize - 2
                elide: Text.ElideMiddle
            }
        }

        Text {
            visible: text.length > 0
            text: row.hintText(row.item.hint ?? "")
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize - 1
        }
    }

    MouseArea {
        id: rowMouse
        anchors.fill: parent
        hoverEnabled: true
        onPositionChanged: mouse => row.pointerMoved(mapToGlobal(mouse.x, mouse.y))
        onClicked: mouse => row.chosen(mouse.modifiers)
    }
}
