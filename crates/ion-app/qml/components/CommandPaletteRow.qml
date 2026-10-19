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

    function iconFor(kind) {
        switch (kind) {
        case "tab": return "tabs"
        case "bookmark": return "star"
        case "history": return "history"
        case "session": return "layers"
        case "setting": return "settings"
        case "command": return "chevron-right"
        case "open": return "globe"
        case "agent": return "agent"
        case "answer": return "copy"
        default: return "search"
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

    // Command hints are portable key text ("Ctrl+…"); show them the
    // platform's way (⌘ on macOS).
    function hintText(hint) {
        return row.item.kind === "command" && hint.length > 0 ? Keymap.display(hint) : hint
    }

    implicitHeight: Theme.tabHeight + Theme.spacing * 2
    radius: Theme.radius
    // One highlight only: pointing at a row selects it, like the arrow keys.
    color: current ? Theme.hoverOverlay : "transparent"

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spacing * 2
        anchors.rightMargin: Theme.spacing * 2
        spacing: Theme.spacing * 2

        // The site's icon, or the kind of result, in a soft tile.
        Rectangle {
            implicitWidth: Theme.iconSize + Theme.spacing * 2
            implicitHeight: implicitWidth
            radius: Theme.radiusSmall
            color: row.current ? Theme.accentSoft : Theme.hoverOverlay

            Image {
                id: favicon
                anchors.centerIn: parent
                width: Theme.iconSize
                height: Theme.iconSize
                sourceSize: Qt.size(Theme.iconSize, Theme.iconSize)
                source: row.pageUrl.length > 0 ? "image://favicon/" + row.pageUrl : ""
                // Unknown icons load as an empty image.
                visible: status === Image.Ready && implicitWidth > 0
            }
            Icon {
                anchors.centerIn: parent
                visible: !favicon.visible
                name: row.iconFor(row.item.kind)
                size: Theme.iconSize - 2
                color: row.current ? Theme.accent : Theme.textMuted
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
                font.weight: row.current ? Theme.weightMedium : Font.Normal
                elide: Text.ElideRight
            }
            Text {
                Layout.fillWidth: true
                visible: (row.item.subtitle ?? "").length > 0
                text: row.marked(row.item.subtitle ?? "", false)
                textFormat: Text.StyledText
                color: Theme.textMuted
                font.pixelSize: Theme.fontSizeSmall
                elide: Text.ElideMiddle
            }
        }

        // Shortcut or action hint, as a key cap.
        Rectangle {
            visible: hint.text.length > 0
            implicitWidth: hint.implicitWidth + Theme.spacing * 2
            implicitHeight: hint.implicitHeight + Theme.spacing / 2
            radius: Theme.radiusSmall
            color: "transparent"
            border.width: Theme.hairline
            border.color: Theme.border

            Text {
                id: hint
                anchors.centerIn: parent
                text: row.hintText(row.item.hint ?? "")
                color: Theme.textMuted
                font.pixelSize: Theme.fontSizeSmall
            }
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
