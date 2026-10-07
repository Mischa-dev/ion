import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// Toolbar button for the current site's remembered permissions (camera,
// location, notifications…). It only shows once the site has a stored
// Allow or Block; its panel switches each one or resets it to "ask again".
IconButton {
    id: control

    property var view   // BrowserTab of the current tab, may be null

    readonly property url origin: view ? Basics.permissionOrigin(view.url) : ""
    // The stored decisions for this site: webEnginePermission values whose
    // state is Granted or Denied.
    property var stored: []
    // Choices made in the panel, by permission type: the engine can keep
    // reporting the old state until the page reloads.
    property var chosen: ({})

    function refresh() {
        if (!view || origin.toString().length === 0) {
            stored = []
            return
        }
        stored = view.profile.listPermissionsForOrigin(origin).filter(
            p => p.state === WebEnginePermission.State.Granted || p.state === WebEnginePermission.State.Denied)
        if (stored.length === 0)
            panel.close()
    }

    onOriginChanged: {
        chosen = {}
        refresh()
    }
    onViewChanged: refresh()
    // The engine applies grant/deny/reset a moment later, so re-read after
    // a short delay rather than at once.
    Connections {
        target: Basics
        function onPermissionsChanged() { refreshLater.restart() }
    }
    Timer {
        id: refreshLater
        interval: 150
        onTriggered: control.refresh()
    }

    visible: stored.length > 0
    glyph: stored.length > 0 ? Basics.permissionGlyph(stored[0].permissionType) : ""
    tip: panel.opened ? "" : qsTr("Site permissions")
    onClicked: panel.opened ? panel.close() : panel.open()

    background: Rectangle {
        radius: Theme.radius
        color: control.down || panel.opened ? Theme.surfaceRaised
             : control.hovered ? Theme.surfaceHover : "transparent"
    }

    Popup {
        id: panel

        // Set once something changed, so the panel offers a reload: a
        // permission already in use keeps working until the page reloads.
        property bool changed: false

        y: control.height + Theme.spacing / 2
        x: control.width - width
        width: Theme.tabMaxWidth + Theme.spacing * 14
        padding: Theme.spacing * 2
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
        onOpened: changed = false

        background: Rectangle {
            radius: Theme.radius
            color: Theme.surface
            border.width: 1
            border.color: Theme.border
        }

        enter: Transition {
            NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
        }
        exit: Transition {
            NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
        }

        function apply(permission, action) {
            if (action === "allow")
                permission.grant()
            else if (action === "block")
                permission.deny()
            else
                permission.reset()
            const chosen = Object.assign({}, control.chosen)
            if (action === "reset")
                delete chosen[permission.permissionType]
            else
                chosen[permission.permissionType] = action === "allow"
            control.chosen = chosen
            changed = true
            Basics.notifyPermissionsChanged()
        }

        contentItem: ColumnLayout {
            spacing: Theme.spacing * 1.5

            Label {
                Layout.fillWidth: true
                text: Basics.permissionPanelHeading(control.origin)
                color: Theme.text
                font.pixelSize: Theme.fontSize
                font.weight: Font.DemiBold
                elide: Text.ElideMiddle
            }

            Repeater {
                model: control.stored

                delegate: RowLayout {
                    id: row
                    required property var modelData
                    readonly property var choice: control.chosen[modelData.permissionType]
                    readonly property bool granted: choice !== undefined ? choice
                        : modelData.state === WebEnginePermission.State.Granted

                    Layout.fillWidth: true
                    spacing: Theme.spacing

                    Text {
                        text: Basics.permissionGlyph(row.modelData.permissionType)
                        font.pixelSize: Theme.fontSize + 2
                    }
                    Label {
                        Layout.fillWidth: true
                        text: Basics.permissionName(row.modelData.permissionType)
                        color: Theme.text
                        font.pixelSize: Theme.fontSize
                        elide: Text.ElideRight
                    }
                    Choice {
                        text: qsTr("Allow")
                        selected: row.granted
                        onClicked: if (!row.granted) panel.apply(row.modelData, "allow")
                    }
                    Choice {
                        text: qsTr("Block")
                        selected: !row.granted
                        onClicked: if (row.granted) panel.apply(row.modelData, "block")
                    }
                    IconButton {
                        implicitWidth: Theme.urlBarHeight * 0.8
                        implicitHeight: implicitWidth
                        glyph: "↺"
                        tip: qsTr("Ask next time")
                        onClicked: panel.apply(row.modelData, "reset")
                    }
                }
            }

            RowLayout {
                visible: panel.changed
                Layout.fillWidth: true
                spacing: Theme.spacing

                Label {
                    Layout.fillWidth: true
                    text: qsTr("Reload to apply to this page")
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSize - 1
                    wrapMode: Text.Wrap
                }
                DialogButton {
                    text: qsTr("Reload")
                    onClicked: {
                        control.view?.reload()
                        panel.close()
                    }
                }
            }
        }
    }

    // A compact Allow / Block toggle; the chosen one is filled.
    component Choice: ToolButton {
        id: choice
        property bool selected: false
        focusPolicy: Qt.NoFocus
        implicitHeight: Theme.urlBarHeight * 0.8
        leftPadding: Theme.spacing * 1.5
        rightPadding: Theme.spacing * 1.5
        contentItem: Text {
            text: choice.text
            color: choice.selected ? Theme.onAccent : Theme.text
            font.pixelSize: Theme.fontSize - 1
            font.weight: choice.selected ? Font.DemiBold : Font.Normal
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
        background: Rectangle {
            radius: Theme.radius
            color: choice.selected ? Theme.accent
                 : choice.hovered ? Theme.surfaceHover : Theme.surfaceRaised
        }
    }
}
