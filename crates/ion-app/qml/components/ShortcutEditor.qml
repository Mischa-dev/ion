import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Keyboard shortcuts: every command with its keys. Search by name or by key,
// click a key to change it, + to add one, × to remove one. Changes are saved
// to `[shortcuts]` in the config's overrides file right away; a key taken
// from another command says so below the list.
Popup {
    id: root

    property var bindings: []
    // The command whose next key press is being recorded, and the key it
    // replaces ("" adds one).
    property string recordingId: ""
    property string replacing: ""
    // What the last change did, shown under the list.
    property string notice: ""

    readonly property int rowHeight: Theme.tabHeight + Theme.spacing * 2
    readonly property var shown: {
        const query = search.text.trim().toLowerCase()
        if (query.length === 0)
            return bindings
        return bindings.filter(b => (b.title + " " + b.keywords + " " + b.id).toLowerCase().includes(query)
                               || b.keys.some(k => k.toLowerCase().includes(query)
                                              || Keymap.display(k).toLowerCase().includes(query)))
    }

    function show() {
        search.text = ""
        notice = ""
        stopRecording()
        open()
        search.forceActiveFocus()
    }

    function refresh() {
        bindings = Keymap.bindings()
    }

    function record(id, key) {
        recordingId = id
        replacing = key
        notice = qsTr("Press the new keys for %1. Esc cancels.").arg(Keymap.title(id))
        recorder.forceActiveFocus()
    }

    function stopRecording() {
        recordingId = ""
        replacing = ""
    }

    function save(id, keys) {
        const error = Keymap.assign(id, keys)
        if (error.length > 0)
            notice = error
        refresh()
    }

    // A key press while recording.
    function recorded(sequence) {
        const id = recordingId
        const binding = bindings.find(b => b.id === id)
        stopRecording()
        search.forceActiveFocus()
        if (!binding)
            return
        const owner = Keymap.owner(sequence)
        const keys = binding.keys.filter(k => k !== replacing && k !== sequence)
        keys.push(sequence)
        save(id, keys)
        notice = owner.length > 0 && owner !== id
            ? qsTr("%1 now runs %2 instead of %3.").arg(Keymap.display(sequence)).arg(binding.title).arg(Keymap.title(owner))
            : qsTr("%1 now runs %2.").arg(Keymap.display(sequence)).arg(binding.title)
    }

    function remove(id, key) {
        const binding = bindings.find(b => b.id === id)
        if (binding)
            save(id, binding.keys.filter(k => k !== key))
        notice = ""
    }

    function resetOne(id) {
        const error = Keymap.reset(id)
        notice = error
        refresh()
    }

    function resetAll() {
        const error = Keymap.resetAll()
        notice = error.length > 0 ? error : qsTr("Every shortcut is back to its default.")
        refresh()
    }

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Theme.urlBarHeight * 2
    width: Math.min(Theme.paletteWidth, parent.width - Theme.spacing * 4)
    padding: Theme.spacing * 2
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    onAboutToShow: refresh()
    onClosed: stopRecording()
    // The config also changes from outside (an edited file, home-manager).
    Connections {
        target: Config
        function onRevisionChanged() { if (root.opened) root.refresh() }
    }

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
        NumberAnimation { property: "scale"; from: 0.97; to: 1; duration: Theme.animationSlowMs; easing.type: Easing.OutCubic }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
    }

    contentItem: ColumnLayout {
        spacing: Theme.spacing

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing

            Text {
                Layout.fillWidth: true
                text: qsTr("Keyboard shortcuts")
                color: Theme.text
                font.pixelSize: Theme.fontSize + 2
                font.weight: Font.DemiBold

            // Takes the keys while recording, before any shortcut can: accepting
            // ShortcutOverride keeps Ctrl+T from opening a tab mid-recording.
            Item {
                id: recorder
                Keys.onShortcutOverride: event => event.accepted = root.recordingId.length > 0
                Keys.onPressed: event => {
                    event.accepted = true
                    if (root.recordingId.length === 0)
                        return
                    if (event.key === Qt.Key_Escape && event.modifiers === Qt.NoModifier) {
                        root.stopRecording()
                        root.notice = ""
                        search.forceActiveFocus()
                        return
                    }
                    const sequence = Keymap.fromEvent(event.key, event.modifiers)
                    if (sequence.length > 0)
                        root.recorded(sequence)
                }
            }
            }
            DialogButton {
                text: qsTr("Reset all")
                visible: root.bindings.some(b => b.customized)
                onClicked: root.resetAll()
            }
        }

        IonTextField {
            id: search
            Layout.fillWidth: true
            placeholderText: qsTr("Search commands or keys")
            Keys.onUpPressed: list.decrementCurrentIndex()
            Keys.onDownPressed: list.incrementCurrentIndex()
            // Enter adds a key to the selected command.
            Keys.onReturnPressed: { const b = root.shown[list.currentIndex]; if (b) root.record(b.id, "") }
            Keys.onEnterPressed: { const b = root.shown[list.currentIndex]; if (b) root.record(b.id, "") }
            onTextChanged: list.currentIndex = 0
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(count, Theme.paletteMaxRows + 3) * root.rowHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            highlightMoveDuration: 0
            model: root.shown
            ScrollBar.vertical: ScrollBar {}

            delegate: Rectangle {
                id: row

                required property var modelData
                required property int index
                readonly property bool recording: root.recordingId === modelData.id

                width: ListView.view.width
                height: root.rowHeight
                radius: Theme.radius
                color: ListView.isCurrentItem || hover.hovered ? Theme.surfaceRaised : "transparent"

                HoverHandler { id: hover }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.spacing * 2
                    anchors.rightMargin: Theme.spacing
                    spacing: Theme.spacing

                    Text {
                        Layout.fillWidth: true
                        text: row.modelData.title
                        color: Theme.text
                        font.pixelSize: Theme.fontSize
                        elide: Text.ElideRight
                    }

                    Repeater {
                        model: row.modelData.keys

                        delegate: KeyChip {
                            required property string modelData
                            label: Keymap.display(modelData)
                            active: row.recording && root.replacing === modelData
                            removable: hover.hovered
                            onClicked: root.record(row.modelData.id, modelData)
                            onRemoveClicked: root.remove(row.modelData.id, modelData)
                        }
                    }

                    KeyChip {
                        // The slot a new key goes into while recording.
                        visible: row.recording && root.replacing.length === 0
                        label: qsTr("Press keys…")
                        active: true
                    }

                    Text {
                        visible: row.modelData.keys.length === 0 && !row.recording
                        text: qsTr("None")
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSize - 1
                    }

                    IconButton {
                        glyph: "+"
                        tip: qsTr("Add a shortcut")
                        opacity: hover.hovered || row.ListView.isCurrentItem ? 1 : 0
                        onClicked: root.record(row.modelData.id, "")
                    }
                    IconButton {
                        glyph: "↺"
                        tip: qsTr("Back to %1").arg(row.modelData.defaults.length > 0
                            ? row.modelData.defaults.map(k => Keymap.display(k)).join(", ") : qsTr("no shortcut"))
                        visible: row.modelData.customized
                        onClicked: root.resetOne(row.modelData.id)
                    }
                }
            }
        }

        Text {
            Layout.fillWidth: true
            visible: list.count === 0
            text: qsTr("No matches")
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize
        }

        Text {
            Layout.fillWidth: true
            text: root.notice.length > 0 ? root.notice
                : qsTr("Click a shortcut to change it. Changes are saved to your config right away.")
            color: root.notice.length > 0 ? Theme.text : Theme.textMuted
            font.pixelSize: Theme.fontSize - 1
            wrapMode: Text.Wrap
        }

        Repeater {
            model: { Config.revision; return Keymap.warnings() }
            delegate: Text {
                required property string modelData
                Layout.fillWidth: true
                text: modelData
                color: Theme.warning
                font.pixelSize: Theme.fontSize - 1
                wrapMode: Text.Wrap
            }
        }
    }
}
