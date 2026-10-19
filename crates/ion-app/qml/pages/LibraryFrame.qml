import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// The frame shared by History, Bookmarks and Downloads: a sidebar to move
// between them, a heading with a search field and actions, and the page's
// own content below.
Rectangle {
    id: frame

    property WebEngineView view
    property string path
    // "history" | "bookmarks" | "downloads"
    property string current
    property string title
    property alias query: search.text
    property alias placeholder: search.placeholderText
    // Buttons beside the search field.
    property alias actions: actionsRow.data
    // Extra content under the sidebar's links (Bookmarks' folders).
    property alias sidebarExtra: extra.data
    default property alias content: body.data

    readonly property var browser: frame.Window.window

    // The find command (Commands.qml) searches the list instead.
    function focusSearch() {
        search.forceActiveFocus()
        search.selectAll()
    }

    color: Theme.background

    RowLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: Theme.tabMaxWidth + Theme.spacing * 6
            color: Theme.surface

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: Theme.spacing * 3
                spacing: Theme.hairline

                Repeater {
                    model: [
                        { id: "history", title: qsTr("History"), icon: "history" },
                        { id: "bookmarks", title: qsTr("Bookmarks"), icon: "bookmark" },
                        { id: "downloads", title: qsTr("Downloads"), icon: "download" }
                    ]
                    delegate: Rectangle {
                        id: link
                        required property var modelData
                        readonly property bool on: modelData.id === frame.current

                        Layout.fillWidth: true
                        implicitHeight: Theme.tabHeight
                        radius: Theme.radius
                        color: on ? Theme.surfaceRaised : linkMouse.containsMouse ? Theme.surfaceHover : "transparent"

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: Theme.spacing * 2
                            anchors.rightMargin: Theme.spacing * 2
                            spacing: Theme.spacing * 2
                            Icon {
                                name: link.modelData.icon
                                color: link.on ? Theme.accent : Theme.textMuted
                            }
                            Text {
                                Layout.fillWidth: true
                                text: link.modelData.title
                                color: Theme.text
                                font.pixelSize: Theme.fontSize
                                font.weight: link.on ? Font.DemiBold : Font.Normal
                            }
                        }
                        MouseArea {
                            id: linkMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: if (!link.on) frame.view.url = Pages.url(link.modelData.id, "")
                        }
                    }
                }

                ColumnLayout {
                    id: extra
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.topMargin: Theme.spacing * 3
                    spacing: Theme.hairline
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Settings")
                    color: settingsMouse.containsMouse ? Theme.text : Theme.textMuted
                    font.pixelSize: Theme.fontSize - 1
                    MouseArea {
                        id: settingsMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: frame.browser.openPage("settings")
                    }
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            // Heading, search and actions, centered over the content.
            Item {
                Layout.fillWidth: true
                implicitHeight: heading.implicitHeight + Theme.spacing * 10

                RowLayout {
                    id: heading
                    x: Math.max(Theme.spacing * 4, (parent.width - width) / 2)
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: Theme.spacing * 3
                    width: Math.min(parent.width - Theme.spacing * 8, Theme.readableWidth + Theme.tabMaxWidth)
                    spacing: Theme.spacing * 2

                    Text {
                        text: frame.title
                        color: Theme.text
                        font.pixelSize: Theme.titleFontSize
                        font.weight: Font.DemiBold
                    }
                    Item { Layout.fillWidth: true }
                    IonTextField {
                        id: search
                        Layout.preferredWidth: Theme.tabMaxWidth * 1.2
                        Keys.onEscapePressed: text = ""
                    }
                    RowLayout {
                        id: actionsRow
                        spacing: Theme.spacing
                    }
                }
            }

            Item {
                id: body
                Layout.fillWidth: true
                Layout.fillHeight: true
            }
        }
    }
}
