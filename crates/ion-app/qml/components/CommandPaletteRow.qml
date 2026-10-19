import QtQuick
import QtQuick.Layouts
import Ion

// One result row from `PaletteSearch`, shared by the command palette and the
// URL bar's suggestions: kind icon, title, subtitle and a right-aligned hint.
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

    readonly property bool mac: Qt.platform.os === "osx" || Qt.platform.os === "macos"

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
        default: return "search"
        }
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
    color: current ? Theme.hoverOverlay : "transparent"

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spacing * 2
        anchors.rightMargin: Theme.spacing * 2
        spacing: Theme.spacing * 2

        // The kind of result, in a soft tile.
        Rectangle {
            implicitWidth: Theme.iconSize + Theme.spacing * 2
            implicitHeight: implicitWidth
            radius: Theme.radiusSmall
            color: row.current ? Theme.accentSoft : Theme.hoverOverlay

            Icon {
                anchors.centerIn: parent
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
                text: row.item.title ?? ""
                color: Theme.text
                font.pixelSize: Theme.fontSize
                font.weight: row.current ? Theme.weightMedium : Font.Normal
                elide: Text.ElideRight
            }
            Text {
                Layout.fillWidth: true
                visible: text.length > 0
                text: row.item.subtitle ?? ""
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
