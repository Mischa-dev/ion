import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// The loaded Chrome extensions: switch each on or off, open its popup in a
// tab, and see why any were skipped. `browser` (the main window) provides
// profile, extensionPaths and openTab(url).
Popup {
    id: root

    required property var browser
    property var items: []
    property var problems: []

    function refresh() {
        const manager = browser.profile?.extensionManager
        // Only the ones Ion loaded; Chromium's built-in component
        // extensions (PDF viewer…) stay hidden.
        const ours = browser.extensionPaths
        items = manager ? manager.extensions.filter(e => ours.includes(e.path.toString())) : []
        problems = Extensions.problems()
    }

    function show() {
        refresh()
        open()
    }

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Theme.urlBarHeight * 3
    width: Math.min(Theme.paletteWidth, parent.width - Theme.spacing * 4)
    padding: Theme.spacing * 2
    modal: true
    focus: true

    palette.window: Theme.surface
    palette.windowText: Theme.text
    palette.base: Theme.surfaceRaised
    palette.text: Theme.text
    palette.button: Theme.surfaceRaised
    palette.buttonText: Theme.text
    palette.light: Theme.surfaceHover
    palette.midlight: Theme.surfaceHover
    palette.mid: Theme.border
    palette.dark: Theme.border
    palette.highlight: Theme.accent
    palette.highlightedText: Theme.onAccent
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    onOpened: Qt.callLater(() => content.forceActiveFocus())

    Connections {
        target: root.browser.profile?.extensionManager ?? null
        ignoreUnknownSignals: true
        function onLoadFinished() { if (root.opened) root.refresh() }
    }

    Overlay.modal: Rectangle {
        color: Qt.rgba(Theme.background.r, Theme.background.g, Theme.background.b, Theme.scrimOpacity)
    }

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

    contentItem: ColumnLayout {
        id: content
        spacing: Theme.spacing
        focus: true
        Keys.onEscapePressed: root.close()

        Text {
            Layout.fillWidth: true
            text: qsTr("Extensions")
            color: Theme.text
            font.pixelSize: Theme.fontSize + 2
            font.bold: true
        }

        Text {
            Layout.fillWidth: true
            visible: root.items.length === 0
            wrapMode: Text.Wrap
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize
            text: qsTr("No extensions yet. Put unpacked Manifest V3 extensions in %1, or list their folders under extensions in your config, then restart Ion.")
                .arg(Extensions.folder)
        }

        Repeater {
            model: root.items

            delegate: Rectangle {
                id: row

                required property var modelData

                Layout.fillWidth: true
                implicitHeight: rowLayout.implicitHeight + Theme.spacing * 2
                radius: Theme.radius
                color: Theme.surfaceRaised

                RowLayout {
                    id: rowLayout
                    anchors.fill: parent
                    anchors.margins: Theme.spacing
                    anchors.leftMargin: Theme.spacing * 2
                    spacing: Theme.spacing

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0

                        Text {
                            Layout.fillWidth: true
                            text: row.modelData.name || row.modelData.id
                            color: Theme.text
                            font.pixelSize: Theme.fontSize
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            text: row.modelData.error || row.modelData.description
                            color: row.modelData.error ? Theme.danger : Theme.textMuted
                            font.pixelSize: Theme.fontSize - 1
                            elide: Text.ElideRight
                            visible: text.length > 0
                        }
                    }

                    IconButton {
                        glyph: "↗"
                        tip: qsTr("Open the extension's popup in a tab")
                        visible: row.modelData.actionPopupUrl.toString().length > 0
                        onClicked: {
                            root.close()
                            root.browser.openTab(row.modelData.actionPopupUrl)
                        }
                    }

                    IonSwitch {
                        checked: row.modelData.isEnabled
                        onToggled: {
                            root.browser.profile.extensionManager.setExtensionEnabled(row.modelData, checked)
                            Extensions.setDisabled(row.modelData.path.toString(), !checked)
                            root.refresh()
                        }
                    }
                }
            }
        }

        Repeater {
            model: root.problems

            delegate: Text {
                required property string modelData
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                text: qsTr("Skipped %1").arg(modelData)
                color: Theme.warning
                font.pixelSize: Theme.fontSize - 1
            }
        }
    }
}
