import QtQuick
import QtQuick.Layouts
import QtWebEngine
import Ion

// The new-tab page: a clock, a greeting and site shortcuts, drawn in Ion's
// theme over an empty tab. Lives inside a BrowserTab and hides as soon as the
// tab navigates somewhere.
Rectangle {
    id: page

    required property WebEngineView view

    property date now: new Date()
    // `{title, url, letter}` for each tile: pinned ones from the config, then
    // the most-visited sites. Rebuilt whenever the page shows.
    property var tiles: []

    function refresh() {
        tiles = JSON.parse(Basics.newTabTiles(History.search("", 200)))
    }

    onVisibleChanged: if (visible) refresh()
    Component.onCompleted: refresh()

    Connections {
        target: Config
        function onRevisionChanged() { page.refresh() }
    }

    visible: view.showNewTabPage && Basics.isNewTabUrl(view.url) && !view.loading
    anchors.fill: parent
    z: 5
    // A shade off the chrome, so the page reads as a card inside it.
    color: Theme.surface

    Timer {
        interval: 1000
        repeat: true
        running: page.visible
        triggeredOnStart: true
        onTriggered: page.now = new Date()
    }

    // A faint wash of the accent from the top, so the page has some depth
    // without competing with the shortcuts.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: parent.height * 0.6
        gradient: Gradient {
            GradientStop { position: 0; color: Theme.alpha(Theme.accent, Theme.dark ? 0.08 : 0.06) }
            GradientStop { position: 1; color: Theme.alpha(Theme.accent, 0) }
        }
    }

    // A letter tile's color: one of the workspace colors, picked by the
    // site's title so it stays the same between visits.
    function tileColor(title) {
        let hash = 0
        for (let i = 0; i < title.length; i++)
            hash = (hash * 31 + title.charCodeAt(i)) | 0
        const colors = Theme.workspaceColors
        return colors[Math.abs(hash) % colors.length].value
    }

    // Scrolls when a short or narrow window can't fit every shortcut.
    Flickable {
        anchors.fill: parent
        contentHeight: Math.max(height, content.implicitHeight + Theme.spacing * 8)
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        ColumnLayout {
            id: content
            anchors.horizontalCenter: parent.horizontalCenter
            // A little above the middle, where the eye lands.
            y: Math.max(Theme.spacing * 4, (parent.height - implicitHeight) * 0.42)
            width: Math.min(page.width - Theme.spacing * 8, Theme.tabMaxWidth * 3.5)
            spacing: 0

            Text {
                Layout.alignment: Qt.AlignHCenter
                // Hours and minutes in the locale's style, without seconds.
                text: Qt.formatTime(page.now, Qt.locale().timeFormat(Locale.ShortFormat).replace(/[:.]ss/, ""))
                color: Theme.text
                font.pixelSize: Theme.displayFontSize
                font.weight: Font.Light
                font.letterSpacing: -Theme.displayFontSize * 0.03
            }

            Text {
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: Theme.spacing
                text: Basics.greeting(page.now.getHours()) + "  ·  "
                      + page.now.toLocaleDateString(Qt.locale(), "dddd d MMMM")
                color: Theme.textMuted
                font.pixelSize: Theme.fontSizeLarge
            }

            GridLayout {
                id: grid
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: Theme.spacing * 8
                columns: Math.max(1, Math.min(page.tiles.length, Math.floor((parent.width + columnSpacing) / (tileSize + columnSpacing))))
                columnSpacing: Theme.spacing * 2
                rowSpacing: Theme.spacing * 2

                readonly property int tileSize: Theme.tabHeight * 3

                Repeater {
                    model: page.tiles

                    delegate: Item {
                        id: tile
                        required property var modelData
                        readonly property bool hovered: tileMouse.containsMouse

                        Layout.preferredWidth: grid.tileSize
                        Layout.preferredHeight: grid.tileSize

                        Rectangle {
                            id: card
                            width: parent.width
                            height: parent.height
                            // Lifts a little under the pointer.
                            y: tile.hovered ? -Theme.hairline * 2 : 0
                            radius: Theme.radiusLarge
                            color: tile.hovered ? Theme.surface : Theme.alpha(Theme.surface, 0.6)
                            border.width: Theme.hairline
                            border.color: tile.hovered ? Theme.border : "transparent"

                            Behavior on y { NumberAnimation { duration: Theme.animationMs; easing.type: Theme.easeOut } }
                            Behavior on color { ColorAnimation { duration: Theme.animationMs } }

                            IonShadow {
                                anchors.fill: parent
                                z: -1
                                radius: parent.radius
                                spread: Theme.spacing * 2
                                offset: Theme.hairline * 3
                                strength: 0.5
                                opacity: tile.hovered ? 1 : 0
                                Behavior on opacity { NumberAnimation { duration: Theme.animationMs } }
                            }

                            ColumnLayout {
                                anchors.centerIn: parent
                                width: parent.width - Theme.spacing * 3
                                spacing: Theme.spacing * 1.5

                                Rectangle {
                                    Layout.alignment: Qt.AlignHCenter
                                    implicitWidth: Theme.tabHeight * 1.4
                                    implicitHeight: implicitWidth
                                    radius: Theme.radius * 1.25
                                    color: favicon.visible ? Theme.surfaceRaised : page.tileColor(tile.modelData.title)

                                    // The site's icon once Ion has seen it, else
                                    // its first letter.
                                    Image {
                                        id: favicon
                                        anchors.centerIn: parent
                                        width: Theme.iconSize * 1.75
                                        height: width
                                        sourceSize: Qt.size(width * 2, height * 2)
                                        source: "image://favicon/" + tile.modelData.iconPage
                                        smooth: true
                                        // Unknown icons load as an empty image.
                                        visible: status === Image.Ready && implicitWidth > 0
                                    }
                                    Text {
                                        anchors.centerIn: parent
                                        visible: !favicon.visible
                                        text: tile.modelData.letter
                                        color: Theme.onSwatch
                                        font.pixelSize: Theme.fontSizeLarge + 3
                                        font.weight: Theme.weightBold
                                    }
                                }

                                Text {
                                    Layout.fillWidth: true
                                    text: tile.modelData.title
                                    color: tile.hovered ? Theme.text : Theme.textMuted
                                    font.pixelSize: Theme.fontSizeSmall + 1
                                    font.weight: Theme.weightMedium
                                    horizontalAlignment: Text.AlignHCenter
                                    elide: Text.ElideRight
                                }
                            }
                        }

                        MouseArea {
                            id: tileMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: page.view.url = tile.modelData.url
                        }
                    }
                }
            }

            Text {
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: Theme.spacing * 6
                text: qsTr("Type to search or enter an address  ·  %1 for commands")
                      .arg(Qt.platform.os === "osx" || Qt.platform.os === "macos" ? "⌘K" : "Ctrl+K")
                color: Theme.textSubtle
                font.pixelSize: Theme.fontSizeSmall
            }
        }
    }
}
