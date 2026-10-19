import QtQuick
import QtQuick.Layouts
import Ion

// One result row from `PaletteSearch`, shared by the command palette and the
// URL bar's suggestions: kind glyph, title, subtitle and a right-aligned hint.
Rectangle {
    id: row

    // `{ kind, title, subtitle, hint, action, value }` from PaletteSearch.
    property var item: ({})
    // Highlighted by the keyboard.
    property bool current: false
    // Clicked, with the keyboard modifiers held.
    signal chosen(int modifiers)
    // The pointer moved over the row; `position` is in global coordinates.
    // Lists select on real movement only, so rows sliding under a resting
    // pointer don't steal the keyboard selection.
    signal pointerMoved(point position)

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
        default: return "⌕"
        }
    }

    // Command hints are portable key text ("Ctrl+…"); show them the
    // platform's way (⌘ on macOS).
    function hintText(hint) {
        return row.item.kind === "command" && hint.length > 0 ? Keymap.display(hint) : hint
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

        Text {
            Layout.preferredWidth: Theme.iconSize
            text: row.glyph(row.item.kind)
            color: row.current ? Theme.accent : Theme.textMuted
            font.pixelSize: Theme.fontSize + 2
            horizontalAlignment: Text.AlignHCenter
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            Text {
                Layout.fillWidth: true
                text: row.item.title ?? ""
                color: Theme.text
                font.pixelSize: Theme.fontSize
                elide: Text.ElideRight
            }
            Text {
                Layout.fillWidth: true
                visible: text.length > 0
                text: row.item.subtitle ?? ""
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
