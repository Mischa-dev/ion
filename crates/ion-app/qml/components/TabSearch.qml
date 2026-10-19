import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Tab search (Ctrl/Cmd+Shift+A, or the strip's search button): every open tab
// in every workspace, most recently used first, then recently closed tabs.
// Typing narrows both. Enter switches to the tab (and its workspace) or
// reopens the closed one; the × on a row closes that tab.
//
// `browser` is the main window, for closeTab(index) and viewAt(index).
Popup {
    id: root

    required property var browser
    property var results: []
    property point lastPointer

    readonly property int rowHeight: Theme.tabHeight + Theme.spacing * 2

    function show() {
        field.text = ""
        open()
        field.forceActiveFocus()
    }

    function refresh() {
        const keep = results[list.currentIndex]
        results = JSON.parse(Tabs.searchTabs(field.text))
        // The current tab heads a blank search; start on the one before it,
        // so Enter goes back like Ctrl+Tab would.
        const start = field.text.length === 0 && results.length > 1 && results[0].current ? 1 : 0
        const again = keep ? results.findIndex(r => r.kind === keep.kind && r.index === keep.index) : -1
        list.currentIndex = results.length === 0 ? -1 : again >= 0 && field.text.length === 0 ? again : start
        if (list.currentIndex <= start)
            list.positionViewAtBeginning()
    }

    // Results hold row indexes, so redo them when tabs open, close or move
    // while the popup is up (a page can open one in the background).
    Connections {
        target: Tabs
        enabled: root.opened
        function onSessionChanged() { Qt.callLater(root.refresh) }
    }

    function choose(item) {
        if (!item)
            return
        close()
        if (item.kind === "closed")
            Tabs.reopenClosedTabAt(item.index)
        else
            Tabs.activate(item.index)
    }

    function closeRow(item) {
        if (item.kind === "tab")
            root.browser.closeTab(item.index)
        Qt.callLater(refresh)
    }

    // Moving the pointer selects the row under it; a resting pointer doesn't.
    function pointed(index, position) {
        const last = lastPointer
        lastPointer = position
        if (last.x >= 0 && (last.x !== position.x || last.y !== position.y))
            list.currentIndex = index
    }

    function host(url) {
        const rest = url.replace(/^[a-z][a-z0-9+.-]*:\/\//i, "").replace(/^www\./, "")
        return rest.length > 0 ? rest : qsTr("New Tab")
    }

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Theme.urlBarHeight * 3
    width: Math.min(Theme.paletteWidth, parent.width - Theme.spacing * 4)
    padding: Theme.spacing
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    onAboutToShow: {
        lastPointer = Qt.point(-1, -1)
        refresh()
    }
    onOpened: field.forceActiveFocus()

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

    contentItem: ColumnLayout {
        spacing: Theme.spacing

        TextField {
            id: field
            Layout.fillWidth: true
            implicitHeight: Theme.urlBarHeight
            color: Theme.text
            placeholderText: qsTr("Search open and recently closed tabs")
            placeholderTextColor: Theme.textMuted
            selectionColor: Theme.accent
            selectedTextColor: Theme.onAccent
            font.pixelSize: Theme.fontSize
            leftPadding: Theme.spacing * 2
            rightPadding: Theme.spacing * 2
            verticalAlignment: TextInput.AlignVCenter
            selectByMouse: true

            background: Rectangle {
                radius: Theme.radius
                color: Theme.surfaceRaised
            }

            onTextChanged: if (root.opened || root.visible) root.refresh()

            Keys.onUpPressed: list.decrementCurrentIndex()
            Keys.onDownPressed: list.incrementCurrentIndex()
            Keys.onEscapePressed: root.close()
            Keys.onReturnPressed: root.choose(root.results[list.currentIndex])
            Keys.onEnterPressed: root.choose(root.results[list.currentIndex])
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(count, Theme.paletteMaxRows + 2) * root.rowHeight
                                    + (root.results.some(r => r.kind === "closed") ? Theme.tabHeight : 0)
            visible: count > 0
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            highlightMoveDuration: 0
            model: root.results
            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)

            delegate: Item {
                id: slot

                required property var modelData
                required property int index
                // The first closed tab carries the "Recently closed" heading.
                readonly property bool heading: modelData.kind === "closed"
                    && (index === 0 || root.results[index - 1].kind !== "closed")

                width: ListView.view.width
                height: root.rowHeight + (heading ? Theme.tabHeight : 0)

                Text {
                    visible: slot.heading
                    height: Theme.tabHeight
                    leftPadding: Theme.spacing * 2
                    verticalAlignment: Text.AlignBottom
                    bottomPadding: Theme.spacing
                    text: qsTr("Recently closed")
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSize - 1
                    font.bold: true
                }

                Rectangle {
                    id: row

                    readonly property var modelData: slot.modelData
                    readonly property int index: slot.index
                    readonly property bool selected: slot.ListView.isCurrentItem
                    readonly property bool closed: modelData.kind === "closed"
                    readonly property var view: closed ? null : root.browser.viewAt(modelData.index)

                    anchors.bottom: parent.bottom
                    width: parent.width
                    height: root.rowHeight
                    radius: Theme.radius
                    color: selected ? Theme.surfaceRaised : "transparent"

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        onPositionChanged: mouse => root.pointed(row.index, mapToGlobal(mouse.x, mouse.y))
                        onClicked: root.choose(row.modelData)
                    }

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: Theme.spacing * 2
                        anchors.rightMargin: Theme.spacing
                        spacing: Theme.spacing * 2

                        Item {
                            Layout.preferredWidth: Theme.iconSize
                            Layout.preferredHeight: Theme.iconSize
                            TabIcon {
                                anchors.centerIn: parent
                                view: row.view
                                savedIcon: row.modelData.icon
                                opacity: row.modelData.suspended || row.closed ? Theme.dimmedOpacity : 1
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 0

                            Text {
                                Layout.fillWidth: true
                                text: row.modelData.title.length > 0 ? row.modelData.title : root.host(row.modelData.url)
                                textFormat: Text.PlainText
                                color: row.closed ? Theme.textMuted : Theme.text
                                font.pixelSize: Theme.fontSize
                                elide: Text.ElideRight
                            }
                            Text {
                                Layout.fillWidth: true
                                visible: row.modelData.url.length > 0
                                text: root.host(row.modelData.url)
                                textFormat: Text.PlainText
                                color: Theme.textMuted
                                font.pixelSize: Theme.fontSize - 2
                                elide: Text.ElideMiddle
                            }
                        }

                        Text {
                            visible: text.length > 0
                            text: row.modelData.current ? qsTr("Current")
                                : row.modelData.suspended ? qsTr("Sleeping")
                                : row.modelData.pinned && row.modelData.workspace.length === 0 ? qsTr("Essential")
                                : ""
                            color: Theme.textMuted
                            font.pixelSize: Theme.fontSize - 2
                        }

                        // The workspace, when it isn't the one showing.
                        Rectangle {
                            visible: row.modelData.workspace.length > 0 && workspaceCount.value > 1
                            implicitWidth: label.implicitWidth + Theme.spacing * 2
                            implicitHeight: label.implicitHeight + Theme.spacing
                            radius: Theme.radius
                            color: Theme.surfaceHover
                            Text {
                                id: label
                                anchors.centerIn: parent
                                text: row.modelData.workspace
                                textFormat: Text.PlainText
                                color: Theme.textMuted
                                font.pixelSize: Theme.fontSize - 2
                            }
                        }

                        IconButton {
                            Layout.preferredWidth: Theme.tabHeight - Theme.spacing * 2
                            Layout.preferredHeight: Theme.tabHeight - Theme.spacing * 2
                            visible: !row.closed && (row.selected || hovered)
                            iconName: "close"
                            tip: qsTr("Close tab")
                            onClicked: root.closeRow(row.modelData)
                        }
                    }
                }
            }
        }

        // Workspace badges only matter with more than one workspace.
        QtObject {
            id: workspaceCount
            property int value: 1
        }
        Connections {
            target: root
            function onAboutToShow() { workspaceCount.value = JSON.parse(Tabs.workspaces()).length }
        }

        Text {
            Layout.fillWidth: true
            Layout.margins: Theme.spacing
            visible: list.count === 0
            text: qsTr("No tabs match")
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize
        }
    }
}
