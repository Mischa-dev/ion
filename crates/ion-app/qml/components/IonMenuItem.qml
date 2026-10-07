import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// One row of an IonMenu: the label, then a check mark when checked or an arrow
// when it opens a submenu.
MenuItem {
    id: item

    implicitHeight: Theme.tabHeight
    leftPadding: Theme.spacing * 2
    rightPadding: Theme.spacing * 2

    indicator: null
    arrow: null

    contentItem: RowLayout {
        spacing: Theme.spacing

        Text {
            Layout.fillWidth: true
            text: item.text
            color: item.enabled ? Theme.text : Theme.textMuted
            opacity: item.enabled ? 1 : 0.6
            font.pixelSize: Theme.fontSize
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
        }
        Text {
            visible: item.checkable && item.checked
            text: "✓"
            color: Theme.accent
            font.pixelSize: Theme.fontSize
        }
        Text {
            visible: item.subMenu !== null
            text: "›"
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize + 2
        }
    }

    background: Rectangle {
        implicitWidth: Theme.tabMaxWidth - Theme.spacing
        radius: Math.max(0, Theme.radius - 2)
        color: item.highlighted && item.enabled ? Theme.surfaceHover : "transparent"
    }
}
