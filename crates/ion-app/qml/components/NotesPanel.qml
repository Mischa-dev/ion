import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Notes in the side panel: a searchable list, newest first, and an editor
// that saves as you type. Clippings (the page menu's "Clip to Notes", or
// Ctrl+Alt+C) quote the selection and link back to the page.
Item {
    id: root

    // The main window: openTab(url).
    required property var browser

    // The note being edited, or -1 for the list.
    property int editing: -1
    readonly property var notes: { Panels.notesRevision; return Panels.notes(search.text) }

    function edit(id) {
        editing = id
        editor.text = Panels.noteText(id)
        editor.forceActiveFocus()
        editor.cursorPosition = editor.text.length
    }

    function newNote() {
        search.text = ""
        // Saved once something is typed; an empty note never sticks.
        editing = Panels.addNote(" ")
        editor.text = ""
        editor.forceActiveFocus()
    }

    function back() {
        save()
        editing = -1
    }

    function save() {
        saveTimer.stop()
        if (editing >= 0)
            Panels.setNoteText(editing, editor.text)
    }

    // "3 min ago", "Yesterday", or a date.
    function when(seconds) {
        const then = new Date(seconds * 1000)
        const minutes = Math.floor((Date.now() - then.getTime()) / 60000)
        if (minutes < 1) return qsTr("Just now")
        if (minutes < 60) return qsTr("%1 min ago").arg(minutes)
        if (minutes < 24 * 60) return qsTr("%1 h ago").arg(Math.floor(minutes / 60))
        if (minutes < 48 * 60) return qsTr("Yesterday")
        return then.toLocaleDateString(Qt.locale(), Locale.ShortFormat)
    }

    function host(url) {
        const match = /^[a-z]+:\/\/(?:www\.)?([^/]+)/i.exec(url)
        return match ? match[1] : ""
    }

    // A clipping made while the list is showing opens nothing by itself;
    // one made into the note being edited shows up there.
    Connections {
        target: Panels
        function onNotesRevisionChanged() {
            if (root.editing < 0 || editor.activeFocus)
                return
            const text = Panels.noteText(root.editing)
            if (text.length === 0)
                root.editing = -1
            else if (text !== editor.text)
                editor.text = text
        }
    }

    Timer {
        id: saveTimer
        interval: 400
        onTriggered: root.save()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.spacing
        visible: root.editing < 0

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing

            IonTextField {
                id: search
                Layout.fillWidth: true
                placeholderText: qsTr("Search notes")
            }
            IconButton {
                iconName: "plus"
                tip: qsTr("New note")
                onClicked: root.newNote()
            }
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: Theme.spacing / 2
            model: root.notes
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {}

            delegate: Rectangle {
                id: row

                required property var modelData

                width: ListView.view.width
                implicitHeight: column.implicitHeight + Theme.spacing * 2
                radius: Theme.radius
                color: rowHover.hovered ? Theme.surfaceHover : Theme.surfaceRaised

                HoverHandler { id: rowHover; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: root.edit(row.modelData.id) }

                ColumnLayout {
                    id: column
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.spacing
                    anchors.leftMargin: Theme.spacing * 2
                    spacing: Theme.spacing / 2

                    Text {
                        Layout.fillWidth: true
                        text: row.modelData.title || qsTr("Empty note")
                        color: Theme.text
                        font.pixelSize: Theme.fontSize
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        // The rest of the note, on two lines at most.
                        text: row.modelData.text.split("\n").filter(l => l.trim().length > 0).slice(1).join(" ")
                            .replace(/^> /, "").replace(/ > /g, " ")
                        visible: text.length > 0
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSize - 1
                        wrapMode: Text.Wrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        text: [root.host(row.modelData.url), root.when(row.modelData.updated)]
                            .filter(s => s.length > 0).join(" · ")
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSize - 2
                        elide: Text.ElideRight
                    }
                }
            }
        }

        Text {
            Layout.fillWidth: true
            Layout.fillHeight: list.count === 0
            visible: list.count === 0
            wrapMode: Text.Wrap
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize
            text: search.text.length > 0
                ? qsTr("No notes match.")
                : qsTr("No notes yet. Press + to write one, or select text on a page and choose Clip to Notes.")
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.spacing
        visible: root.editing >= 0

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing

            IconButton {
                iconName: "back"
                tip: qsTr("All notes")
                onClicked: root.back()
            }
            Text {
                id: source
                readonly property var note: { Panels.notesRevision; return root.notes.find(n => n.id === root.editing) ?? null }
                Layout.fillWidth: true
                text: note && note.url.length > 0 ? root.host(note.url) : ""
                color: sourceHover.hovered ? Theme.accent : Theme.textMuted
                font.pixelSize: Theme.fontSize - 1
                elide: Text.ElideRight

                HoverHandler { id: sourceHover; cursorShape: Qt.PointingHandCursor; enabled: source.text.length > 0 }
                TapHandler { enabled: source.text.length > 0; onTapped: root.browser.openTab(source.note.url) }
            }
            IconButton {
                iconName: "trash"
                tip: qsTr("Delete note")
                onClicked: {
                    Panels.removeNote(root.editing)
                    root.editing = -1
                }
            }
        }

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true

            TextArea {
                id: editor
                wrapMode: TextEdit.Wrap
                color: Theme.text
                selectionColor: Theme.accent
                selectedTextColor: Theme.onAccent
                placeholderText: qsTr("Write something…")
                placeholderTextColor: Theme.textMuted
                font.pixelSize: Theme.fontSize
                padding: Theme.spacing * 2
                background: Rectangle {
                    radius: Theme.radius
                    color: Theme.surfaceRaised
                }
                onTextChanged: if (activeFocus) saveTimer.restart()
                Keys.onEscapePressed: root.back()
            }
        }
    }
}
