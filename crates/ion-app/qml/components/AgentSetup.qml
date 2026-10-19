import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// "Set up Ion Agent": pick the model service, paste a key (saved in the
// system keyring, never in config), pick a model, check it works, and turn
// Ion Agent on. Opens from the palette, or when someone asks Ion Agent
// before it is on; that question is asked once the setup is saved.
Popup {
    id: root

    // What was asked before Ion Agent was on, to ask once it is.
    property string pendingInput: ""
    property var setup: null
    property var preset: null
    // "", "testing", "saving".
    property string busy: ""
    property bool tested: false
    property bool testOk: false
    property string message: ""
    property var models: []
    // Where the saved setup's key comes from (`Agent.keyChecked`).
    property string keySource: ""

    // Asks `input` once the setup is saved.
    signal ask(string input)

    function show(input) {
        pendingInput = input || ""
        setup = JSON.parse(Agent.setupJson())
        // Not on yet: start from the first service rather than the defaults.
        const saved = setup.enabled ? setup.presets.find(p => p.id === setup.preset) : null
        choose(saved || setup.presets[0], saved !== null)
        keySource = ""
        Agent.checkKey()
        open()
    }

    // Pick a service. `current` keeps the saved address and model.
    function choose(p, current) {
        preset = p
        const same = current || p.id === setup.preset
        baseField.text = same ? setup.baseUrl : p.baseUrl
        modelField.text = same && setup.model.length > 0 ? setup.model : p.model
        keyField.text = ""
        models = []
        tested = false
        message = ""
    }

    function test() {
        busy = "testing"
        message = ""
        Agent.testSetup(preset.id, baseField.text, modelField.text, keyField.text)
    }

    function save() {
        if (busy.length > 0)
            return
        busy = "saving"
        message = ""
        Agent.saveSetup(preset.id, baseField.text, modelField.text, keyField.text)
    }

    // The saved setup is the one being edited: its key may already be there.
    readonly property bool editingSaved: setup !== null && preset !== null && preset.id === setup.preset
        && baseField.text.trim().replace(/\/+$/, "") === setup.baseUrl.replace(/\/+$/, "")
    readonly property bool hasKey: keyField.text.trim().length > 0
        || (editingSaved && (keySource === "keyring" || keySource === "environment"))
    readonly property bool canSave: preset !== null && busy.length === 0
        && modelField.text.trim().length > 0
        && (preset.needsKey ? hasKey : true)
        && (preset.id !== "custom" || baseField.text.trim().length > 0)

    Connections {
        target: Agent
        function onSetupRequested(input) { root.show(input) }
        function onSetupTested(ok, message, models) {
            if (root.busy !== "testing")
                return
            root.busy = ""
            root.tested = true
            root.testOk = ok
            root.message = message
            root.models = JSON.parse(models)
            // Nothing chosen yet: start from the first the service offers.
            if (ok && modelField.text.trim().length === 0 && root.models.length > 0)
                modelField.text = root.models[0]
        }
        function onSetupSaved(error) {
            if (root.busy !== "saving")
                return
            root.busy = ""
            if (error.length > 0) {
                root.testOk = false
                root.tested = true
                root.message = error
                return
            }
            const input = root.pendingInput
            root.close()
            if (input.length > 0)
                root.ask(input)
        }
        function onKeyChecked(source, error) {
            root.keySource = source
            if (error.length > 0) {
                root.tested = true
                root.testOk = false
                root.message = error
            }
        }
    }

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Theme.urlBarHeight * 3
    width: Math.min(Theme.paletteWidth, parent.width - Theme.spacing * 4)
    padding: Theme.spacing * 3
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

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

    // Opened from the palette, which hands focus back as it closes.
    onOpened: Qt.callLater(() => (root.preset && root.preset.needsKey ? keyField : modelField).forceActiveFocus())

    component Label: Text {
        color: Theme.textMuted
        font.pixelSize: Theme.fontSize - 1
    }

    contentItem: ColumnLayout {
        id: content
        spacing: Theme.spacing * 2
        Keys.onEscapePressed: root.close()

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing / 2
            Text {
                text: qsTr("Set up Ion Agent")
                color: Theme.text
                font.pixelSize: Theme.fontSize + 2
                font.bold: true
            }
            Text {
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                text: root.pendingInput.length > 0
                    ? qsTr("Ion Agent answers from the address bar and works in your tabs, asking before anything that matters. Pick the model it runs on, and your question goes as soon as it's set up.")
                    : qsTr("Ion Agent answers from the address bar and works in your tabs, asking before anything that matters. Pick the model it runs on.")
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize
            }
        }

        // The services, two to a row.
        GridLayout {
            Layout.fillWidth: true
            columns: 2
            columnSpacing: Theme.spacing
            rowSpacing: Theme.spacing

            Repeater {
                model: root.setup ? root.setup.presets : []
                delegate: Rectangle {
                    id: card
                    required property var modelData
                    readonly property bool chosen: root.preset !== null && root.preset.id === modelData.id
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.preferredWidth: 1
                    implicitHeight: cardColumn.implicitHeight + Theme.spacing * 3
                    radius: Theme.radius
                    color: chosen ? Theme.surfaceRaised : cardMouse.containsMouse ? Theme.surfaceHover : "transparent"
                    border.width: Theme.hairline
                    border.color: chosen ? Theme.accent : Theme.border

                    ColumnLayout {
                        id: cardColumn
                        anchors.fill: parent
                        anchors.margins: Theme.spacing * 1.5
                        anchors.leftMargin: Theme.spacing * 2
                        spacing: 2
                        RowLayout {
                            spacing: Theme.spacing
                            Text {
                                text: card.modelData.name
                                color: Theme.text
                                font.pixelSize: Theme.fontSize
                                font.bold: card.chosen
                            }
                            Text {
                                visible: card.modelData.local
                                text: qsTr("on this computer")
                                color: Theme.success
                                font.pixelSize: Theme.fontSize - 2
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            text: card.modelData.blurb
                            color: Theme.textMuted
                            font.pixelSize: Theme.fontSize - 2
                            wrapMode: Text.Wrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                        }
                    }
                    MouseArea {
                        id: cardMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.choose(card.modelData, false)
                    }
                }
            }
        }

        // The service's address: only asked for when it isn't known.
        ColumnLayout {
            Layout.fillWidth: true
            visible: root.preset !== null && (root.preset.id === "custom" || root.preset.local)
            spacing: Theme.spacing / 2
            Label { text: qsTr("Address, up to and including /v1") }
            IonTextField {
                id: baseField
                Layout.fillWidth: true
                placeholderText: "http://127.0.0.1:8080/v1"
                onTextEdited: root.tested = false
                onAccepted: root.save()
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            visible: root.preset !== null && (root.preset.needsKey || root.preset.id === "custom")
            spacing: Theme.spacing / 2
            RowLayout {
                Layout.fillWidth: true
                Label {
                    Layout.fillWidth: true
                    text: !root.preset ? ""
                        : root.editingSaved && root.keySource === "keyring" ? qsTr("API key: saved in your keyring. Paste a new one to replace it.")
                        : root.editingSaved && root.keySource === "environment" ? qsTr("API key: from %1. Paste one to save it in your keyring instead.").arg(root.setup.keyEnv)
                        : root.preset.id === "custom" ? qsTr("API key, if the service needs one")
                        : qsTr("API key, saved in your system keyring")
                }
                Text {
                    visible: root.preset !== null && root.preset.keyPage.length > 0 && root.preset.needsKey
                    text: qsTr("Get a key")
                    color: Theme.accent
                    font.pixelSize: Theme.fontSize - 1
                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            Tabs.openTab(root.preset.keyPage, true)
                            root.close()
                        }
                    }
                }
            }
            IonTextField {
                id: keyField
                Layout.fillWidth: true
                echoMode: TextInput.Password
                placeholderText: qsTr("Paste the key")
                onTextEdited: root.tested = false
                onAccepted: root.save()
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing / 2
            Label { text: qsTr("Model") }
            IonTextField {
                id: modelField
                Layout.fillWidth: true
                placeholderText: qsTr("Model id")
                onAccepted: root.save()
            }
            // What the service offers, once checked: pick one.
            Flow {
                Layout.fillWidth: true
                visible: root.models.length > 0
                spacing: Theme.spacing / 2
                Repeater {
                    // The ones matching what is typed, a handful at a time.
                    model: {
                        const typed = modelField.text.trim().toLowerCase()
                        const matching = root.models.filter(m => typed.length === 0 || m.toLowerCase().includes(typed) || m === modelField.text)
                        return (matching.length > 0 ? matching : root.models).slice(0, 12)
                    }
                    delegate: DialogButton {
                        required property string modelData
                        implicitHeight: Theme.urlBarHeight - Theme.spacing
                        text: modelData
                        primary: modelData === modelField.text
                        onClicked: modelField.text = modelData
                    }
                }
            }
        }

        Text {
            Layout.fillWidth: true
            visible: text.length > 0
            text: root.busy === "testing" ? qsTr("Checking…")
                : root.busy === "saving" ? qsTr("Saving…")
                : root.tested ? root.message : ""
            color: root.busy.length > 0 ? Theme.textMuted : root.testOk ? Theme.success : Theme.danger
            font.pixelSize: Theme.fontSize
            wrapMode: Text.Wrap
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing
            Text {
                Layout.fillWidth: true
                text: qsTr("Pages you ask about are sent to the service you pick.")
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize - 2
                wrapMode: Text.Wrap
            }
            DialogButton {
                text: qsTr("Check")
                enabled: root.preset !== null && root.busy.length === 0
                onClicked: root.test()
            }
            DialogButton {
                text: root.pendingInput.length > 0 ? qsTr("Turn on and ask") : qsTr("Turn on")
                primary: true
                enabled: root.canSave
                onClicked: root.save()
            }
        }
    }
}
