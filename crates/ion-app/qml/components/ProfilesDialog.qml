import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// "Switch profile": each profile is its own Ion window with separate
// sign-ins, cookies, history, bookmarks and passwords (work and personal
// accounts side by side). Also opens private windows.
Popup {
    id: root

    property var names: []
    property string current: Privacy.windowKind() === "profile" ? Privacy.profileName() : ""
    property string error: ""

    function show() {
        names = Privacy.profiles()
        error = ""
        nameField.text = ""
        open()
    }

    function openProfile(name) {
        if (Privacy.openProfile(name))
            close()
        else
            error = qsTr("Use letters, digits, spaces, - and _ (up to 40).")
    }

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Theme.urlBarHeight * 3
    width: Math.min(Theme.paletteWidth * 0.8, parent.width - Theme.spacing * 4)
    padding: Theme.spacing * 3
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    onOpened: Qt.callLater(() => nameField.forceActiveFocus())

    Overlay.modal: Rectangle {
        color: Qt.rgba(Theme.background.r, Theme.background.g, Theme.background.b, Theme.scrimOpacity)
    }
    background: Rectangle {
        radius: Theme.radius
        color: Theme.surface
        border.width: Theme.hairline
        border.color: Theme.border
    }
    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
    }

    component Row: AbstractButton {
        id: row
        property string detail
        property bool here: false
        Layout.fillWidth: true
        implicitHeight: Theme.tabHeight + Theme.spacing * 2
        leftPadding: Theme.spacing * 2
        rightPadding: Theme.spacing * 2
        focusPolicy: Qt.StrongFocus
        enabled: !here
        Keys.onReturnPressed: clicked()
        Keys.onEnterPressed: clicked()

        contentItem: RowLayout {
            spacing: Theme.spacing
            Rectangle {
                implicitWidth: Theme.iconSize + Theme.spacing
                implicitHeight: implicitWidth
                radius: width / 2
                color: Theme.surfaceRaised
                Text {
                    anchors.centerIn: parent
                    text: row.text.charAt(0).toUpperCase()
                    color: Theme.text
                    font.pixelSize: Theme.fontSize - 1
                    font.weight: Font.DemiBold
                }
            }
            Text {
                Layout.fillWidth: true
                text: row.text
                textFormat: Text.PlainText
                color: Theme.text
                font.pixelSize: Theme.fontSize
                elide: Text.ElideRight
            }
            Text {
                text: row.here ? qsTr("This window") : row.detail
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize - 1
            }
        }
        background: Rectangle {
            radius: Theme.radius
            color: row.down || row.hovered ? Theme.surfaceHover : Theme.surfaceRaised
            border.width: row.activeFocus ? Theme.hairline : 0
            border.color: Theme.accent
        }
        HoverHandler { cursorShape: row.here ? Qt.ArrowCursor : Qt.PointingHandCursor }
    }

    contentItem: ColumnLayout {
        spacing: Theme.spacing

        Text {
            Layout.fillWidth: true
            text: qsTr("Profiles")
            color: Theme.text
            font.pixelSize: Theme.fontSize + 2
            font.bold: true
        }
        Text {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize
            text: qsTr("Each profile opens in its own window with its own sign-ins, cookies, history, bookmarks and passwords. Settings are shared.")
        }

        Row {
            text: qsTr("Default")
            here: Privacy.windowKind() === "normal"
            onClicked: root.openProfile("")
        }
        Repeater {
            model: root.names
            delegate: Row {
                required property string modelData
                text: modelData
                here: modelData === root.current
                onClicked: root.openProfile(modelData)
            }
        }
        Row {
            text: qsTr("Private window")
            detail: Privacy.windowKind() === "private" ? qsTr("Another one") : qsTr("Ctrl+Shift+N")
            onClicked: {
                Privacy.openPrivateWindow([])
                root.close()
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: Theme.spacing
            spacing: Theme.spacing
            IonTextField {
                id: nameField
                Layout.fillWidth: true
                placeholderText: qsTr("New profile name, like Work")
                onAccepted: if (text.trim().length > 0) root.openProfile(text)
            }
            DialogButton {
                primary: true
                text: qsTr("Create")
                enabled: nameField.text.trim().length > 0
                onClicked: root.openProfile(nameField.text)
            }
        }
        Text {
            Layout.fillWidth: true
            visible: root.error.length > 0
            text: root.error
            color: Theme.danger
            font.pixelSize: Theme.fontSize - 1
            wrapMode: Text.Wrap
        }
    }
}
