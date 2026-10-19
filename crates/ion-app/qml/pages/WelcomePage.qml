import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// ion://welcome: first-run setup. Opens on Ion's first start (and from the
// palette), and walks through look, search and importing from another
// browser. Every choice applies live, saved like any other setting.
Rectangle {
    id: page

    property WebEngineView view
    property string path

    property int step: 0
    readonly property int steps: 5
    // Settings rows from `Preferences.page`, by key, for the choices offered.
    property var settings: ({})
    property var sources: []
    // Browser name to its import result line.
    property var imported: ({})

    readonly property var browser: page.Window.window

    function refresh() {
        const byKey = {}
        const sections = JSON.parse(Preferences.page("", Theme.themeIds ? Theme.themeIds : []))
        for (const section of sections)
            for (const group of section.groups)
                for (const setting of group.settings)
                    byKey[setting.key] = setting
        settings = byKey
    }

    function choose(key, value) {
        Config.set(key, value)
    }

    function choosePreset(key, index) {
        for (const [k, v] of settings[key].control.options[index].values)
            Config.set(k, v)
    }

    function importFrom(name) {
        const bookmarks = Bookmarks.importFrom(name)
        const pages = History.importFrom(name)
        const results = Object.assign({}, imported)
        if (bookmarks < 0 && pages < 0) {
            results[name] = qsTr("Couldn't read it. Close %1 and try again.").arg(name)
        } else {
            History.save()
            results[name] = qsTr("%1 bookmarks and %2 pages imported")
                .arg(Math.max(bookmarks, 0)).arg(Math.max(pages, 0))
        }
        imported = results
    }

    // How wide a centered Flow of `count` tiles is, wrapping within `width`.
    readonly property int tileWidth: Theme.tabMaxWidth * 0.75
    function rowWidth(count, width) {
        const step = tileWidth + Theme.spacing * 2
        const perRow = Math.max(1, Math.min(count, Math.floor((width + Theme.spacing * 2) / step)))
        return perRow * step - Theme.spacing * 2
    }

    function finish() {
        Pages.finishWelcome()
        view.url = Config.homePage
    }

    color: Theme.background
    Component.onCompleted: {
        refresh()
        sources = Bookmarks.importSources()
    }
    Connections {
        target: Config
        function onRevisionChanged() { page.refresh() }
    }

    Keys.onRightPressed: if (step < steps - 1) step++
    Keys.onLeftPressed: if (step > 0) step--

    // A big choosable tile: a picture on top, a label below.
    component Tile: Rectangle {
        id: tile
        property string label
        property bool on: false
        default property alias picture: pictureBox.data
        signal picked()

        implicitWidth: page.tileWidth
        implicitHeight: implicitWidth * 0.75
        radius: Theme.radius * 1.5
        color: tileMouse.containsMouse ? Theme.surfaceRaised : Theme.surface
        border.width: on ? Theme.hairline * 2 : Theme.hairline
        border.color: on ? Theme.accent : Theme.border

        Behavior on border.color { ColorAnimation { duration: Theme.animationMs } }

        Item {
            id: pictureBox
            anchors.fill: parent
            anchors.margins: Theme.spacing * 2
            anchors.bottomMargin: Theme.spacing * 2 + tileLabel.height + Theme.spacing
        }
        Text {
            id: tileLabel
            anchors.bottom: parent.bottom
            anchors.bottomMargin: Theme.spacing * 2
            anchors.horizontalCenter: parent.horizontalCenter
            width: parent.width - Theme.spacing * 2
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideRight
            text: (tile.on ? "✓  " : "") + tile.label
            color: tile.on ? Theme.accent : Theme.text
            font.pixelSize: Theme.fontSize
            font.weight: tile.on ? Font.DemiBold : Font.Normal
        }
        MouseArea {
            id: tileMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: tile.picked()
        }
    }

    component Heading: ColumnLayout {
        property string title
        property string subtitle
        Layout.fillWidth: true
        spacing: Theme.spacing
        Text {
            Layout.fillWidth: true
            text: parent.title
            color: Theme.text
            font.pixelSize: Theme.titleFontSize + 6
            font.weight: Font.DemiBold
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
        }
        Text {
            Layout.fillWidth: true
            Layout.maximumWidth: Theme.readableWidth
            Layout.alignment: Qt.AlignHCenter
            text: parent.subtitle
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize + 2
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
        }
    }

    component Label: Text {
        Layout.topMargin: Theme.spacing * 3
        Layout.alignment: Qt.AlignHCenter
        color: Theme.textMuted
        font.pixelSize: Theme.fontSize - 1
        font.weight: Font.DemiBold
        font.capitalization: Font.AllUppercase
        font.letterSpacing: 0.5
    }

    Flickable {
        id: flick
        anchors.fill: parent
        anchors.bottomMargin: footer.height
        // The current step's own height: the stack is as tall as its tallest.
        readonly property real stepHeight: steps.children[page.step] ? steps.children[page.step].implicitHeight : 0
        contentHeight: Math.max(height, stepHeight + Theme.spacing * 16)
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        StackLayout {
            id: steps
            anchors.horizontalCenter: parent.horizontalCenter
            y: Math.max(Theme.spacing * 8, (flick.height - flick.stepHeight) / 2)
            width: Math.min(page.width - Theme.spacing * 8, Theme.readableWidth * 1.6)
            currentIndex: page.step

            // 0: hello
            ColumnLayout {
                spacing: Theme.spacing * 3
                Image {
                    Layout.alignment: Qt.AlignHCenter
                    source: "qrc:/qt/qml/Ion/icons/dev.ion.Ion.svg"
                    sourceSize: Qt.size(Theme.tabHeight * 3, Theme.tabHeight * 3)
                }
                Heading {
                    title: qsTr("Welcome to Ion")
                    subtitle: qsTr("A quick, quiet browser that fits your desktop. Two minutes to make it yours, or skip and change anything later in Settings.")
                }
                RowLayout {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.topMargin: Theme.spacing * 3
                    spacing: Theme.spacing * 2
                    DialogButton { text: qsTr("Skip setup"); onClicked: page.finish() }
                    DialogButton { text: qsTr("Get started"); primary: true; onClicked: page.step = 1 }
                }
            }

            // 1: look
            ColumnLayout {
                spacing: Theme.spacing * 2
                Heading {
                    title: qsTr("Make it yours")
                    subtitle: qsTr("Pick a theme and where your tabs go. It changes as you click.")
                }
                Label { text: qsTr("Theme") }
                Flow {
                    id: themeFlow
                    readonly property int count: page.settings["theme.name"] ? page.settings["theme.name"].control.options.length : 0
                    Layout.alignment: Qt.AlignHCenter
                    Layout.preferredWidth: page.rowWidth(count, steps.width)
                    spacing: Theme.spacing * 2
                    Repeater {
                        model: page.settings["theme.name"] ? page.settings["theme.name"].control.options : []
                        delegate: Tile {
                            required property var modelData
                            label: modelData.label
                            on: Config.themeSource === "builtin" && Config.themeName === modelData.value
                            implicitHeight: implicitWidth * 0.55
                            onPicked: {
                                page.choose("theme.source", "builtin")
                                page.choose("theme.name", modelData.value)
                            }
                            // A tiny window in the theme's colors; "Automatic"
                            // shows its dark and light halves.
                            readonly property var swatches: Preferences.themeSwatches(modelData.value)
                            readonly property var colors: swatches.length === 4 ? swatches
                                : [Theme.background, Theme.surface, Theme.accent, Theme.text]
                            Rectangle {
                                anchors.fill: parent
                                radius: Theme.radius
                                color: parent.parent.colors[0]
                                border.width: Theme.hairline
                                border.color: Theme.border
                                Rectangle {
                                    x: Theme.spacing; y: Theme.spacing
                                    width: parent.width - Theme.spacing * 2; height: Theme.spacing * 2
                                    radius: 2
                                    color: parent.parent.parent.colors[1]
                                }
                                Rectangle {
                                    x: Theme.spacing; y: Theme.spacing * 4
                                    width: parent.width * 0.5; height: Theme.spacing
                                    radius: height / 2
                                    color: parent.parent.parent.colors[3]
                                    opacity: 0.8
                                }
                                Rectangle {
                                    anchors.right: parent.right; anchors.bottom: parent.bottom
                                    anchors.margins: Theme.spacing
                                    width: Theme.iconSize; height: Theme.spacing * 2
                                    radius: height / 2
                                    color: parent.parent.parent.colors[2]
                                }
                            }
                        }
                    }
                }
                Label { text: qsTr("Tabs") }
                RowLayout {
                    Layout.alignment: Qt.AlignHCenter
                    spacing: Theme.spacing * 3
                    Tile {
                        label: qsTr("Across the top")
                        on: Config.tabLayout === "horizontal"
                        onPicked: page.choose("ui.tabs", "horizontal")
                        Column {
                            anchors.fill: parent
                            spacing: Theme.spacing / 2
                            Row {
                                spacing: Theme.spacing / 2
                                Repeater {
                                    model: 3
                                    Rectangle { width: Theme.iconSize * 2; height: Theme.spacing * 2; radius: 2; color: index === 0 ? Theme.accent : Theme.surfaceHover; required property int index }
                                }
                            }
                            Rectangle { width: parent.width; height: parent.height - Theme.spacing * 3; radius: 3; color: Theme.surfaceRaised }
                        }
                    }
                    Tile {
                        label: qsTr("In a sidebar")
                        on: Config.tabLayout === "vertical"
                        onPicked: page.choose("ui.tabs", "vertical")
                        Row {
                            anchors.fill: parent
                            spacing: Theme.spacing / 2
                            Column {
                                spacing: Theme.spacing / 2
                                Repeater {
                                    model: 4
                                    Rectangle { width: Theme.iconSize * 2; height: Theme.spacing * 2; radius: 2; color: index === 0 ? Theme.accent : Theme.surfaceHover; required property int index }
                                }
                            }
                            Rectangle { width: parent.width - Theme.iconSize * 2 - Theme.spacing / 2; height: parent.height; radius: 3; color: Theme.surfaceRaised }
                        }
                    }
                }
            }

            // 2: search and blocking
            ColumnLayout {
                spacing: Theme.spacing * 2
                Heading {
                    title: qsTr("Search the way you like")
                    subtitle: qsTr("Words typed in the address bar go here. Type !g, !w or !yt first to search one site directly.")
                }
                Flow {
                    readonly property int count: page.settings["search.engine"] ? page.settings["search.engine"].control.options.length : 0
                    Layout.alignment: Qt.AlignHCenter
                    Layout.preferredWidth: page.rowWidth(count, steps.width)
                    Layout.topMargin: Theme.spacing * 2
                    spacing: Theme.spacing * 2
                    Repeater {
                        model: page.settings["search.engine"] ? page.settings["search.engine"].control.options : []
                        delegate: Tile {
                            required property var modelData
                            required property int index
                            label: modelData.label
                            implicitHeight: Theme.tabHeight * 2
                            on: page.settings["search.engine"].selected === index
                            onPicked: page.choosePreset("search.engine", index)
                        }
                    }
                }
                Rectangle {
                    Layout.fillWidth: true
                    Layout.topMargin: Theme.spacing * 4
                    implicitHeight: blockRow.implicitHeight + Theme.spacing * 4
                    radius: Theme.radius * 1.5
                    color: Theme.surface
                    border.width: Theme.hairline
                    border.color: Theme.border
                    RowLayout {
                        id: blockRow
                        anchors.fill: parent
                        anchors.margins: Theme.spacing * 2
                        spacing: Theme.spacing * 2
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.hairline
                            Text { text: qsTr("Block ads and trackers"); color: Theme.text; font.pixelSize: Theme.fontSize }
                            Text {
                                Layout.fillWidth: true
                                text: qsTr("On by default, with the same lists as uBlock Origin. Pages load faster and follow you less.")
                                color: Theme.textMuted
                                font.pixelSize: Theme.fontSize - 1
                                wrapMode: Text.Wrap
                            }
                        }
                        IonSwitch {
                            checked: Config.revision >= 0 && Config.value("adblock.enable") === true
                            onToggled: page.choose("adblock.enable", checked)
                        }
                    }
                }
            }

            // 3: import
            ColumnLayout {
                spacing: Theme.spacing * 2
                Heading {
                    title: qsTr("Bring your bookmarks and history")
                    subtitle: page.sources.length > 0
                        ? qsTr("Ion found these browsers on this computer. Nothing leaves your machine.")
                        : qsTr("Ion didn't find another browser's profile here. You can import later from the palette.")
                }
                Repeater {
                    model: page.sources
                    delegate: Rectangle {
                        id: source
                        required property string modelData
                        readonly property string result: page.imported[modelData] || ""

                        Layout.fillWidth: true
                        Layout.maximumWidth: Theme.readableWidth
                        Layout.alignment: Qt.AlignHCenter
                        implicitHeight: Theme.tabHeight * 2
                        radius: Theme.radius * 1.5
                        color: Theme.surface
                        border.width: Theme.hairline
                        border.color: result.length > 0 ? Theme.accent : Theme.border

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: Theme.spacing * 3
                            anchors.rightMargin: Theme.spacing * 2
                            spacing: Theme.spacing * 2
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: Theme.hairline
                                Text { text: source.modelData; color: Theme.text; font.pixelSize: Theme.fontSize + 1 }
                                Text {
                                    visible: source.result.length > 0
                                    text: source.result
                                    color: Theme.textMuted
                                    font.pixelSize: Theme.fontSize - 1
                                }
                            }
                            DialogButton {
                                text: source.result.length > 0 ? qsTr("Import again") : qsTr("Import")
                                primary: source.result.length === 0
                                onClicked: page.importFrom(source.modelData)
                            }
                        }
                    }
                }
            }

            // 4: ready
            ColumnLayout {
                spacing: Theme.spacing * 2
                Heading {
                    title: qsTr("You're set")
                    subtitle: qsTr("A few keys worth knowing. Every command is in the palette.")
                }
                GridLayout {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.topMargin: Theme.spacing * 2
                    columns: 2
                    columnSpacing: Theme.spacing * 4
                    rowSpacing: Theme.spacing * 1.5
                    Repeater {
                        // Command ids, shown with the keys Keymap has for them now.
                        model: [
                            ["palette", qsTr("Command palette: tabs, history, bookmarks, settings")],
                            ["focus-url", qsTr("Address bar; try !gh, !w or !yt before a search")],
                            ["bookmark-page", qsTr("Bookmark this page")],
                            ["history", qsTr("History")],
                            ["settings", qsTr("Settings")],
                            ["reopen-tab", qsTr("Reopen the tab you just closed")]
                        ]
                        delegate: RowLayout {
                            required property var modelData
                            Layout.columnSpan: 2
                            spacing: Theme.spacing * 3
                            Rectangle {
                                Layout.preferredWidth: Theme.tabMaxWidth * 0.5
                                implicitHeight: keyText.implicitHeight + Theme.spacing
                                radius: Theme.radius
                                color: Theme.surfaceRaised
                                Text {
                                    id: keyText
                                    anchors.centerIn: parent
                                    text: {
                                        Config.revision
                                        const keys = Keymap.keys(modelData[0])
                                        return keys.length > 0 ? Keymap.display(keys[0]) : "—"
                                    }
                                    color: Theme.text
                                    font.pixelSize: Theme.fontSize - 1
                                    font.family: "monospace"
                                }
                            }
                            Text {
                                text: modelData[1]
                                color: Theme.text
                                font.pixelSize: Theme.fontSize
                            }
                        }
                    }
                }
                DialogButton {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.topMargin: Theme.spacing * 4
                    text: qsTr("Start browsing")
                    primary: true
                    onClicked: page.finish()
                }
            }
        }
    }

    // Progress dots and Back/Next, from the second step on.
    Item {
        id: footer
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.tabHeight * 3
        visible: page.step > 0

        Row {
            anchors.centerIn: parent
            spacing: Theme.spacing * 1.5
            Repeater {
                model: page.steps
                delegate: Rectangle {
                    required property int index
                    width: index === page.step ? Theme.spacing * 4 : Theme.spacing * 1.5
                    height: Theme.spacing * 1.5
                    radius: height / 2
                    color: index === page.step ? Theme.accent : Theme.surfaceHover
                    Behavior on width { NumberAnimation { duration: Theme.animationMs } }
                    MouseArea {
                        anchors.fill: parent
                        anchors.margins: -Theme.spacing
                        cursorShape: Qt.PointingHandCursor
                        onClicked: page.step = index
                    }
                }
            }
        }
        DialogButton {
            anchors.left: parent.left
            anchors.leftMargin: Theme.spacing * 6
            anchors.verticalCenter: parent.verticalCenter
            text: qsTr("Back")
            onClicked: page.step--
        }
        DialogButton {
            anchors.right: parent.right
            anchors.rightMargin: Theme.spacing * 6
            anchors.verticalCenter: parent.verticalCenter
            visible: page.step < page.steps - 1
            text: qsTr("Next")
            primary: true
            onClicked: page.step++
        }
    }
}
