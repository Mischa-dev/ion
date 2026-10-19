import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// ion://settings: every setting in the config, by section, with search.
// What is on the page comes from `Preferences.page` (crates/ion-settings);
// each row writes through `Config.set`, which saves to overrides.toml.
Rectangle {
    id: page

    property WebEngineView view
    // The section in the URL: ion://settings/privacy.
    property string path

    property string query: ""
    // The sections shown, with each setting's current state.
    property var sections: []
    // The section the sidebar highlights: the one at the top of the view.
    property string currentSection: sections.length > 0 ? sections[0].id : ""

    readonly property var browser: page.Window.window

    function refresh() {
        const themes = Theme.themeIds ? Theme.themeIds : []
        sections = JSON.parse(Preferences.page(query, themes))
    }

    function scrollTo(id) {
        for (let i = 0; i < sectionRepeater.count; i++) {
            const item = sectionRepeater.itemAt(i)
            if (item && item.sectionId === id) {
                scroller.contentY = Math.min(item.y, Math.max(0, scroller.contentHeight - scroller.height))
                currentSection = id
                return
            }
        }
    }

    // Keep the highlighted section in step with scrolling.
    function updateCurrent() {
        let current = ""
        for (let i = 0; i < sectionRepeater.count; i++) {
            const item = sectionRepeater.itemAt(i)
            if (item && item.y <= scroller.contentY + Theme.spacing * 8)
                current = item.sectionId
        }
        if (current.length === 0 && sections.length > 0)
            current = sections[0].id
        if (scroller.contentY >= scroller.contentHeight - scroller.height - 1 && sections.length > 0)
            current = sections[sections.length - 1].id
        currentSection = current
    }

    // Actions from `Control::Action` rows.
    function run(action) {
        if (action.startsWith("open:")) {
            browser.openTab(action.slice(5))
            return
        }
        switch (action) {
        case "open-config-folder":
            Qt.openUrlExternally("file://" + Config.directory)
            break
        case "copy-nix":
            clipboard.text = Config.overridesAsNix()
            clipboard.selectAll()
            clipboard.copy()
            toast.show(qsTr("Copied. Paste it into programs.ion in your home-manager config."))
            break
        case "reset-all":
            resetDialog.open()
            break
        case "update-filter-lists":
            Adblock.updateLists()
            toast.show(qsTr("Updating filter lists…"))
            break
        case "command-palette":
            browser.showPalette()
            break
        case "clear-browsing-data":
            browser.showClearData()
            break
        default:
            console.warn("Settings: unknown action", action)
        }
    }

    color: Theme.background

    onQueryChanged: refresh()
    onPathChanged: Qt.callLater(() => scrollTo(path))
    Component.onCompleted: {
        refresh()
        if (path.length > 0)
            Qt.callLater(() => scrollTo(path))
    }

    Connections {
        target: Config
        function onRevisionChanged() { page.refresh() }
    }
    Connections {
        target: Theme
        function onThemeIdsChanged() { page.refresh() }
    }

    // For copying text; never shown.
    TextEdit { id: clipboard; visible: false }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // Sidebar: title, search and the sections.
        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: Theme.tabMaxWidth + Theme.spacing * 6
            color: Theme.surface

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: Theme.spacing * 3
                spacing: Theme.spacing

                Text {
                    text: qsTr("Settings")
                    color: Theme.text
                    font.pixelSize: Theme.titleFontSize
                    font.weight: Font.DemiBold
                    Layout.bottomMargin: Theme.spacing * 2
                }

                IonTextField {
                    id: search
                    Layout.fillWidth: true
                    Layout.bottomMargin: Theme.spacing * 2
                    placeholderText: qsTr("Search settings")
                    onTextChanged: page.query = text
                    Keys.onEscapePressed: text = ""
                }

                ListView {
                    id: nav
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    spacing: Theme.hairline
                    model: page.sections
                    boundsBehavior: Flickable.StopAtBounds

                    delegate: Rectangle {
                        id: navItem
                        required property var modelData
                        readonly property bool current: modelData.id === page.currentSection

                        width: nav.width
                        height: Theme.tabHeight
                        radius: Theme.radius
                        color: current ? Theme.surfaceRaised : navMouse.containsMouse ? Theme.surfaceHover : "transparent"

                        Behavior on color { ColorAnimation { duration: Theme.animationMs } }

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: Theme.spacing * 2
                            anchors.rightMargin: Theme.spacing * 2
                            spacing: Theme.spacing * 2

                            Text {
                                Layout.preferredWidth: Theme.iconSize
                                text: navItem.modelData.icon
                                color: navItem.current ? Theme.accent : Theme.textMuted
                                font.pixelSize: Theme.fontSize + 1
                                horizontalAlignment: Text.AlignHCenter
                            }
                            Text {
                                Layout.fillWidth: true
                                text: navItem.modelData.title
                                color: Theme.text
                                font.pixelSize: Theme.fontSize
                                font.weight: navItem.current ? Font.DemiBold : Font.Normal
                                elide: Text.ElideRight
                            }
                        }

                        MouseArea {
                            id: navMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: page.scrollTo(navItem.modelData.id)
                        }
                    }
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Saved to %1").arg(Config.directory + "/overrides.toml")
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSize - 2
                    wrapMode: Text.WrapAnywhere
                }
            }
        }

        Flickable {
            id: scroller
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentHeight: column.implicitHeight + Theme.spacing * 16
            boundsBehavior: Flickable.StopAtBounds
            clip: true
            onContentYChanged: page.updateCurrent()

            ScrollBar.vertical: ScrollBar {}

            ColumnLayout {
                id: column
                x: Math.max(Theme.spacing * 4, (scroller.width - width) / 2)
                y: Theme.spacing * 6
                width: Math.min(scroller.width - Theme.spacing * 8, Theme.readableWidth + Theme.tabMaxWidth)
                spacing: Theme.spacing * 2

                // The config files couldn't be used, or have keys Ion doesn't know.
                Rectangle {
                    Layout.fillWidth: true
                    visible: Config.error.length > 0 || Config.warnings.length > 0
                    implicitHeight: problems.implicitHeight + Theme.spacing * 4
                    radius: Theme.radius
                    color: "transparent"
                    border.width: Theme.hairline
                    border.color: Config.error.length > 0 ? Theme.danger : Theme.warning

                    ColumnLayout {
                        id: problems
                        anchors.fill: parent
                        anchors.margins: Theme.spacing * 2
                        spacing: Theme.spacing

                        Text {
                            Layout.fillWidth: true
                            text: Config.error.length > 0
                                ? qsTr("Your config has a problem, so Ion is using the last settings that worked")
                                : qsTr("Your config has settings Ion doesn't use")
                            color: Config.error.length > 0 ? Theme.danger : Theme.warning
                            font.pixelSize: Theme.fontSize
                            font.weight: Font.DemiBold
                            wrapMode: Text.Wrap
                        }
                        Text {
                            Layout.fillWidth: true
                            text: [Config.error].concat(Config.warnings).filter(s => s.length > 0).join("\n")
                            color: Theme.textMuted
                            font.pixelSize: Theme.fontSize - 1
                            font.family: "monospace"
                            wrapMode: Text.WrapAnywhere
                            textFormat: Text.PlainText
                        }
                    }
                }

                Text {
                    Layout.fillWidth: true
                    Layout.topMargin: Theme.spacing * 8
                    visible: page.sections.length === 0
                    text: qsTr("No settings match “%1”").arg(page.query)
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSize + 2
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                }

                Repeater {
                    id: sectionRepeater
                    model: page.sections

                    delegate: ColumnLayout {
                        id: sectionItem
                        required property var modelData
                        readonly property string sectionId: modelData.id

                        Layout.fillWidth: true
                        Layout.topMargin: Theme.spacing * 4
                        spacing: Theme.spacing * 2

                        Text {
                            Layout.fillWidth: true
                            text: sectionItem.modelData.title
                            color: Theme.text
                            font.pixelSize: Theme.fontSize + 5
                            font.weight: Font.DemiBold
                        }
                        Text {
                            Layout.fillWidth: true
                            Layout.topMargin: -Theme.spacing
                            visible: text.length > 0 && page.query.length === 0
                            text: sectionItem.modelData.summary
                            color: Theme.textMuted
                            font.pixelSize: Theme.fontSize
                            wrapMode: Text.Wrap
                        }

                        Repeater {
                            model: sectionItem.modelData.groups

                            delegate: ColumnLayout {
                                id: groupItem
                                required property var modelData
                                Layout.fillWidth: true
                                spacing: Theme.spacing

                                Text {
                                    visible: text.length > 0
                                    Layout.topMargin: Theme.spacing
                                    text: groupItem.modelData.title
                                    color: Theme.textMuted
                                    font.pixelSize: Theme.fontSize - 1
                                    font.weight: Font.DemiBold
                                    font.capitalization: Font.AllUppercase
                                    font.letterSpacing: 0.5
                                }

                                // A card of rows, divided by hairlines.
                                Rectangle {
                                    Layout.fillWidth: true
                                    implicitHeight: rows.implicitHeight
                                    radius: Theme.radius * 1.5
                                    color: Theme.surface
                                    border.width: Theme.hairline
                                    border.color: Theme.border

                                    ColumnLayout {
                                        id: rows
                                        anchors.left: parent.left
                                        anchors.right: parent.right
                                        spacing: 0

                                        Repeater {
                                            model: groupItem.modelData.settings

                                            delegate: SettingRow {
                                                required property var modelData
                                                required property int index
                                                Layout.fillWidth: true
                                                setting: modelData
                                                first: index === 0
                                                onAction: action => page.run(action)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    PageNotice { id: toast }

    IonDialog {
        id: resetDialog
        title: qsTr("Reset every change made in Settings?")
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: {
            const error = Preferences.resetAll()
            toast.show(error.length > 0 ? error : qsTr("Settings reset to your config file and Ion's defaults."))
        }

        Text {
            width: parent.width
            text: qsTr("Ion goes back to what config.toml says, and to its defaults for the rest. Your config.toml isn't touched.")
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize
            wrapMode: Text.Wrap
        }
    }

    // Ctrl+F searches the settings, like a page's find bar.
    Shortcut {
        sequences: [StandardKey.Find]
        enabled: page.view !== null && page.view.visible
        onActivated: {
            search.forceActiveFocus()
            search.selectAll()
        }
    }
}
