import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// One row of an IonMenu: an optional icon (`iconName`, from IconSet.qml), the
// label, then a check mark when checked or an arrow when it opens a submenu.
MenuItem {
    id: item

    property string iconName

    implicitHeight: Theme.tabHeight
    leftPadding: Theme.spacing * 2
    rightPadding: Theme.spacing * 2

    indicator: null
    arrow: null

    contentItem: RowLayout {
        spacing: Theme.spacing * 1.5

        Icon {
            // Blank, holding the column, in menus that have icons.
            visible: item.iconName.length > 0 || (item.menu?.icons ?? false)
            name: item.iconName
            color: item.highlighted ? Theme.text : Theme.textMuted
        }
        Text {
            Layout.fillWidth: true
            text: item.text
            // Plain text: workspace names and icons in menus are typed by the
            // user, and markup such as <b> in them must show as typed.
            textFormat: Text.PlainText
            color: item.enabled ? Theme.text : Theme.textMuted
            opacity: item.enabled ? 1 : 0.6
            font.pixelSize: Theme.fontSize
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
        }
        Icon {
            visible: item.checkable && item.checked
            name: "check"
            color: Theme.accent
        }
        Icon {
            visible: item.subMenu !== null
            name: "chevron-right"
            size: Theme.iconSize - 2
            color: Theme.textMuted
        }
    }

    background: Rectangle {
        implicitWidth: Theme.tabMaxWidth
        radius: Theme.radiusSmall
        color: item.highlighted && item.enabled ? Theme.hoverOverlay : "transparent"
    }
}
