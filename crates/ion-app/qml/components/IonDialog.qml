import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// A small modal dialog in Ion's look, centered over the window. Set `title`,
// `standardButtons` and put the content inside, as with Dialog.
Dialog {
    id: dialog

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(Theme.paletteWidth * 0.7, parent ? parent.width - Theme.spacing * 4 : Theme.paletteWidth)
    modal: true
    focus: true
    padding: Theme.spacing * 3
    topPadding: Theme.spacing

    Overlay.modal: Rectangle {
        color: Qt.rgba(Theme.background.r, Theme.background.g, Theme.background.b, Theme.scrimOpacity)
    }

    background: Rectangle {
        radius: Theme.radius
        color: Theme.surface
        border.color: Theme.border
        border.width: Theme.hairline
    }

    header: Text {
        text: dialog.title
        visible: text.length > 0
        color: Theme.text
        font.pixelSize: Theme.fontSize + 2
        font.weight: Font.DemiBold
        padding: Theme.spacing * 3
        bottomPadding: Theme.spacing
    }

    footer: DialogButtonBox {
        alignment: Qt.AlignRight
        spacing: Theme.spacing
        padding: Theme.spacing * 3
        topPadding: 0
        background: null

        delegate: Button {
            id: button
            // The accept button carries the accent, the others stay quiet.
            readonly property bool primary: DialogButtonBox.buttonRole === DialogButtonBox.AcceptRole
            implicitHeight: Theme.urlBarHeight
            leftPadding: Theme.spacing * 3
            rightPadding: Theme.spacing * 3
            focusPolicy: Qt.NoFocus

            contentItem: Text {
                text: button.text
                color: button.primary ? Theme.onAccent : Theme.text
                font.pixelSize: Theme.fontSize
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            background: Rectangle {
                radius: Theme.radius
                color: button.primary ? Theme.accent
                     : button.down ? Theme.surfaceRaised : button.hovered ? Theme.surfaceHover : Theme.surfaceRaised
                opacity: button.primary && button.down ? 0.85 : 1
            }
        }
    }

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
        NumberAnimation { property: "scale"; from: 0.97; to: 1; duration: Theme.animationSlowMs; easing.type: Easing.OutCubic }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
    }
}
