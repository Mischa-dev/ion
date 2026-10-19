import QtQuick
import QtQuick.Controls
import Ion

// A drop-down for the print preview's settings, in Ion's colors. `model` is
// a list of strings.
ComboBox {
    id: control

    implicitHeight: Theme.urlBarHeight
    font.pixelSize: Theme.fontSize
    leftPadding: Theme.spacing * 2

    contentItem: Text {
        text: control.displayText
        color: control.enabled ? Theme.text : Theme.textMuted
        font: control.font
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
        rightPadding: control.indicator.width + Theme.spacing
    }

    indicator: Text {
        x: control.width - width - Theme.spacing * 2
        y: (control.height - height) / 2
        text: "▾"
        color: Theme.textMuted
        font.pixelSize: Theme.fontSize
    }

    background: Rectangle {
        radius: Theme.radius
        color: control.hovered ? Theme.surfaceHover : Theme.surfaceRaised
        border.width: control.visualFocus ? Theme.hairline * 2 : Theme.hairline
        border.color: control.visualFocus ? Theme.accent : Theme.border
    }

    delegate: ItemDelegate {
        id: row
        required property int index
        required property var modelData
        width: control.width
        height: Theme.urlBarHeight
        highlighted: control.highlightedIndex === index
        contentItem: Text {
            text: row.modelData
            color: Theme.text
            font.pixelSize: Theme.fontSize
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }
        background: Rectangle {
            radius: Theme.radius
            color: row.highlighted ? Theme.surfaceHover : "transparent"
        }
    }

    popup: Popup {
        y: control.height + Theme.spacing / 2
        width: control.width
        implicitHeight: Math.min(contentItem.implicitHeight + padding * 2, Theme.urlBarHeight * 8)
        padding: Theme.spacing / 2
        contentItem: ListView {
            clip: true
            implicitHeight: contentHeight
            model: control.popup.visible ? control.delegateModel : null
            currentIndex: control.highlightedIndex
        }
        background: Rectangle {
            radius: Theme.radius
            color: Theme.surface
            border.width: Theme.hairline
            border.color: Theme.border
        }
    }
}
