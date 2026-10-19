import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// ion://bookmarks: every bookmark, by folder, with search, editing and an
// undo for removals.
LibraryFrame {
    id: page


    // "*" for every bookmark, "" for those in no folder, else a folder path.
    property string folder: "*"
    property var folders: []
    property var items: []
    // The last bookmark removed, for Undo.
    property var removed: null

    function refresh() {
        folders = JSON.parse(Bookmarks.folders())
        if (folder !== "*" && folder !== "" && !folders.some(f => f.path === folder))
            folder = "*"
        items = query.length > 0 || folder === "*"
            ? JSON.parse(Bookmarks.search(query, 5000))
            : JSON.parse(Bookmarks.inFolder(folder))
    }

    function openItem(item, newTab) {
        if (newTab)
            browser.openTab(item.url, false)
        else
            view.url = item.url
    }

    function remove(item) {
        removed = item
        Bookmarks.remove(item.url)
        undoTimer.restart()
    }

    function undo() {
        if (!removed)
            return
        Bookmarks.toggle(removed.url, removed.title)
        if (removed.folder.length > 0)
            Bookmarks.edit(removed.url, removed.url, removed.title, removed.folder)
        removed = null
    }

    current: "bookmarks"
    title: qsTr("Bookmarks")
    placeholder: qsTr("Search bookmarks")
    onQueryChanged: refresh()
    onFolderChanged: refresh()
    Component.onCompleted: refresh()

    Connections {
        target: Bookmarks
        function onRevisionChanged() { page.refresh() }
    }

    actions: [
        DialogButton {
            text: qsTr("Import…")
            onClicked: page.browser.showImport()
        }
    ]

    // The folders, in the sidebar under the page links.
    sidebarExtra: [
        Text {
            Layout.leftMargin: Theme.spacing * 2
            Layout.bottomMargin: Theme.spacing
            text: qsTr("FOLDERS")
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize - 2
            font.weight: Font.DemiBold
            font.letterSpacing: 0.5
        },
        ListView {
            id: folderList
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            model: [{ path: "*", name: qsTr("All bookmarks"), depth: 0, count: Bookmarks.count },
                    { path: "", name: qsTr("Not in a folder"), depth: 0, count: -1 }].concat(page.folders)

            delegate: Rectangle {
                id: folderRow
                required property var modelData
                readonly property bool on: page.query.length === 0 && modelData.path === page.folder

                width: folderList.width
                height: Theme.tabHeight
                radius: Theme.radius
                color: on ? Theme.surfaceRaised : folderMouse.containsMouse ? Theme.surfaceHover : "transparent"

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.spacing * 2 + folderRow.modelData.depth * Theme.spacing * 3
                    anchors.rightMargin: Theme.spacing * 2
                    spacing: Theme.spacing * 2
                    Text {
                        text: folderRow.modelData.path === "*" ? "☆" : "▸"
                        color: folderRow.on ? Theme.accent : Theme.textMuted
                        font.pixelSize: Theme.fontSize - 1
                    }
                    Text {
                        Layout.fillWidth: true
                        text: folderRow.modelData.name
                        color: Theme.text
                        font.pixelSize: Theme.fontSize
                        elide: Text.ElideRight
                        textFormat: Text.PlainText
                    }
                    Text {
                        visible: folderRow.modelData.count > 0
                        text: folderRow.modelData.count
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSize - 2
                    }
                }
                MouseArea {
                    id: folderMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    cursorShape: Qt.PointingHandCursor
                    onClicked: mouse => {
                        page.query = ""
                        page.folder = folderRow.modelData.path
                        if (mouse.button === Qt.RightButton && folderRow.modelData.depth >= 0
                                && folderRow.modelData.path.length > 0 && folderRow.modelData.path !== "*") {
                            folderMenu.path = folderRow.modelData.path
                            folderMenu.popup()
                        }
                    }
                }
            }
        }
    ]

    ListView {
        id: list
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spacing * 4
        anchors.horizontalCenter: parent.horizontalCenter
        width: Math.min(parent.width - Theme.spacing * 8, Theme.readableWidth + Theme.tabMaxWidth)
        clip: true
        focus: true
        keyNavigationEnabled: true
        currentIndex: -1
        highlightMoveDuration: 0
        boundsBehavior: Flickable.StopAtBounds
        model: page.items
        ScrollBar.vertical: ScrollBar {}

        Keys.onReturnPressed: event => page.openItem(page.items[currentIndex], event.modifiers & Qt.ControlModifier)
        Keys.onDeletePressed: page.remove(page.items[currentIndex])

        delegate: LibraryRow {
            required property var modelData
            required property int index

            width: ListView.view.width
            url: modelData.url
            title: modelData.title
            // Where it lives, when the list mixes folders.
            note: page.folder === "*" || page.query.length > 0 ? modelData.folder.split("/").pop() : ""
            selected: ListView.isCurrentItem && list.activeFocus
            removeTip: qsTr("Delete bookmark")
            editTip: qsTr("Edit")
            onOpen: newTab => page.openItem(modelData, newTab)
            onRemoveRequested: page.remove(modelData)
            onEditRequested: editDialog.edit(modelData)
            onMenuRequested: {
                list.currentIndex = index
                itemMenu.item = modelData
                itemMenu.popup()
            }
        }
    }

    Text {
        anchors.centerIn: parent
        visible: page.items.length === 0
        text: page.query.length > 0 ? qsTr("No bookmarks match “%1”").arg(page.query)
            : Bookmarks.count === 0 ? qsTr("Press Ctrl+D on any page to bookmark it, or import your bookmarks from another browser.")
            : qsTr("This folder is empty")
        width: Math.min(parent.width - Theme.spacing * 8, Theme.readableWidth)
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.Wrap
        color: Theme.textMuted
        font.pixelSize: Theme.fontSize + 2
    }

    // "Deleted “…”  Undo", for a few seconds after a removal.
    Rectangle {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spacing * 4
        width: undoRow.implicitWidth + Theme.spacing * 4
        height: Theme.tabHeight + Theme.spacing
        radius: Theme.radius
        color: Theme.surfaceRaised
        border.width: Theme.hairline
        border.color: Theme.border
        opacity: page.removed !== null ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: Theme.animationMs } }

        RowLayout {
            id: undoRow
            anchors.centerIn: parent
            spacing: Theme.spacing * 3
            Text {
                Layout.maximumWidth: Theme.readableWidth * 0.7
                text: page.removed ? qsTr("Deleted “%1”").arg(page.removed.title || page.removed.url) : ""
                color: Theme.text
                font.pixelSize: Theme.fontSize
                elide: Text.ElideRight
                textFormat: Text.PlainText
            }
            Text {
                text: qsTr("Undo")
                color: Theme.accent
                font.pixelSize: Theme.fontSize
                font.weight: Font.DemiBold
                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -Theme.spacing
                    cursorShape: Qt.PointingHandCursor
                    onClicked: page.undo()
                }
            }
        }
    }
    Timer {
        id: undoTimer
        interval: Theme.toastMs
        onTriggered: page.removed = null
    }

    IonMenu {
        id: itemMenu
        property var item: ({})
        IonMenuItem {
            text: qsTr("Open in new tab")
            onTriggered: page.browser.openTab(itemMenu.item.url, false)
        }
        IonMenuItem {
            text: qsTr("Edit…")
            onTriggered: editDialog.edit(itemMenu.item)
        }
        IonMenuSeparator {}
        IonMenuItem {
            text: qsTr("Delete")
            onTriggered: page.remove(itemMenu.item)
        }
    }

    IonMenu {
        id: folderMenu
        property string path
        IonMenuItem {
            text: qsTr("Rename…")
            onTriggered: renameDialog.rename(folderMenu.path)
        }
        IonMenuItem {
            text: qsTr("Delete folder and its bookmarks")
            onTriggered: {
                Bookmarks.removeFolder(folderMenu.path)
                page.folder = "*"
            }
        }
    }

    IonDialog {
        id: editDialog
        property string original

        function edit(item) {
            original = item.url
            titleField.text = item.title
            urlField.text = item.url
            folderField.text = item.folder
            error.text = ""
            open()
            titleField.forceActiveFocus()
        }

        title: qsTr("Edit bookmark")
        standardButtons: Dialog.Save | Dialog.Cancel
        onAccepted: {
            const problem = Bookmarks.edit(original, urlField.text, titleField.text, folderField.text)
            if (problem.length > 0) {
                error.text = problem
                open()
            }
        }

        ColumnLayout {
            width: parent.width
            spacing: Theme.spacing
            Text { text: qsTr("Name"); color: Theme.textMuted; font.pixelSize: Theme.fontSize - 1 }
            IonTextField { id: titleField; Layout.fillWidth: true; onAccepted: editDialog.accept() }
            Text { text: qsTr("Address"); color: Theme.textMuted; font.pixelSize: Theme.fontSize - 1 }
            IonTextField { id: urlField; Layout.fillWidth: true; onAccepted: editDialog.accept() }
            Text { text: qsTr("Folder (use / for folders inside folders)"); color: Theme.textMuted; font.pixelSize: Theme.fontSize - 1 }
            IonTextField { id: folderField; Layout.fillWidth: true; onAccepted: editDialog.accept() }
            Text {
                id: error
                Layout.fillWidth: true
                visible: text.length > 0
                color: Theme.danger
                font.pixelSize: Theme.fontSize - 1
                wrapMode: Text.Wrap
            }
        }
    }

    IonDialog {
        id: renameDialog
        property string path

        function rename(folder) {
            path = folder
            nameField.text = folder
            open()
            nameField.forceActiveFocus()
            nameField.selectAll()
        }

        title: qsTr("Rename folder")
        standardButtons: Dialog.Save | Dialog.Cancel
        onAccepted: {
            Bookmarks.renameFolder(path, nameField.text)
            page.folder = nameField.text.split("/").map(s => s.trim()).filter(s => s.length > 0).join("/")
        }

        IonTextField { id: nameField; width: parent.width; onAccepted: renameDialog.accept() }
    }
}
