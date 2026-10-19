import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// The header of a tab group in either strip: its color and name. Clicking
// folds or unfolds the group; right-click opens the group menu.
Rectangle {
    id: chip

    required property int group
    required property string name
    required property string groupColor
    required property bool collapsed
    // Number of tabs, shown while folded.
    property int tabs: 0
    // Full-width row in the sidebar rather than a pill in the strip.
    property bool vertical: false
    // The sidebar is narrowed to icons: only the color shows.
    property bool compact: false

    readonly property color tint: groupColor.length > 0 ? groupColor : Theme.accent

    signal toggled()
    signal menuRequested(Item anchor)

    implicitWidth: compact ? Theme.tabHeight : vertical ? 0 : Math.min(Theme.tabMaxWidth * 2 / 3, row.implicitWidth + Theme.spacing * 3)
    implicitHeight: vertical ? Theme.tabHeight - Theme.spacing : Theme.tabHeight - Theme.spacing * 2
    radius: Theme.radius
    color: mouse.containsMouse ? Theme.surfaceHover : vertical ? "transparent" : Qt.rgba(tint.r, tint.g, tint.b, Theme.tintOpacity)
    border.width: vertical ? 0 : Theme.hairline
    border.color: tint

    Behavior on color { ColorAnimation { duration: Theme.animationMs } }

    RowLayout {
        id: row
        anchors.fill: parent
        anchors.leftMargin: chip.vertical ? Theme.spacing * 2 : Theme.spacing * 1.5
        anchors.rightMargin: Theme.spacing * 1.5
        spacing: Theme.spacing

        Rectangle {
            Layout.alignment: Qt.AlignVCenter
            visible: chip.vertical || chip.name.length === 0
            implicitWidth: Theme.workspaceDotSize
            implicitHeight: Theme.workspaceDotSize
            radius: width / 2
            color: chip.tint
        }
        Text {
            Layout.fillWidth: chip.vertical
            Layout.maximumWidth: chip.vertical ? -1 : Theme.tabMaxWidth * 2 / 3 - Theme.spacing * 6
            visible: !chip.compact && chip.name.length > 0
            text: chip.name
            textFormat: Text.PlainText
            color: Theme.text
            font.pixelSize: Theme.fontSize - 1
            font.bold: true
            elide: Text.ElideRight
        }
        Text {
            Layout.alignment: Qt.AlignVCenter
            Layout.fillWidth: chip.vertical && chip.name.length === 0
            visible: !chip.compact && chip.collapsed && chip.tabs > 0
            text: chip.vertical && chip.name.length === 0 ? qsTr("%n tab(s)", "", chip.tabs) : chip.tabs
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize - 1
        }
        Text {
            Layout.alignment: Qt.AlignVCenter
            visible: chip.vertical && !chip.compact
            text: chip.collapsed ? "▸" : "▾"
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize
        }
    }

    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onClicked: event => {
            if (event.button === Qt.RightButton)
                chip.menuRequested(chip)
            else
                chip.toggled()
        }
    }

    ToolTip.visible: mouse.containsMouse && (chip.compact || chip.name.length === 0)
    ToolTip.text: chip.collapsed ? qsTr("Show tabs (%1)").arg(chip.tabs) : qsTr("Fold group")
    ToolTip.delay: 600
}
