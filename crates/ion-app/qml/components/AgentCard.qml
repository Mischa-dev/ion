import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// What an agent is doing for the current tab, dropped from the address bar:
// the question, a short log of steps, any approval it needs, and the
// answer. Opened when the person asks with `@ion …` or `!ai …`; Esc or a
// click elsewhere puts it away (the address bar's agent chip brings it back
// while the agent works).
Popup {
    id: card

    // The `Agent` task shown, or -1.
    property int taskId: -1
    // The tab the person is looking at, which Take over takes back, and
    // what it shows, for follow-ups.
    property int tabId: -1
    property string pageUrl: ""
    property string pageTitle: ""
    // How tall the conversation gets before it scrolls.
    property real maxBodyHeight: 480
    readonly property var task: {
        Agent.revision
        const json = taskId >= 0 ? Agent.taskJson(taskId) : ""
        return json.length > 0 ? JSON.parse(json) : null
    }
    readonly property bool working: task !== null
        && (task.status === "thinking" || task.status === "working")
    readonly property var prompt: task ? task.prompt : null
    // The agent has been changing the page, and whether the person took the
    // tab back from it.
    readonly property bool acting: task !== null && task.acting
    readonly property bool takenOver: task !== null && task.takenOver
    // Emitted when the card closes, so the page can take focus back.
    signal finished()

    function openFor(id) {
        taskId = id
        open()
    }

    // Allow stays disabled for a moment after a prompt appears, so a click or
    // key already on its way can't land on it (docs/SAFETY.md).
    property bool armed: false
    onPromptChanged: {
        armed = false
        if (prompt)
            armTimer.restart()
    }
    Timer {
        id: armTimer
        interval: Theme.promptArmMs
        onTriggered: card.armed = true
    }

    function answer(choice) {
        if (!prompt || (choice !== "deny" && !armed))
            return
        Agent.answer(taskId, choice)
    }

    // Once it has answered, typing goes to the follow-up field.
    onWorkingChanged: if (!working && opened && task) followUp.forceActiveFocus()
    onOpened: if (!working && task) followUp.forceActiveFocus()

    // A finished task is forgotten once put away; a working one keeps going
    // behind the chip.
    onClosed: {
        if (task && !working)
            Agent.dismiss(taskId)
        finished()
    }

    padding: Theme.spacing * 3
    focus: true
    closePolicy: Popup.CloseOnPressOutsideParent

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
        NumberAnimation { property: "y"; from: card.y - Theme.spacing * 2; to: card.y; duration: Theme.animationMs; easing.type: Easing.OutCubic }
    }

    background: Rectangle {
        radius: Theme.radius + 2
        color: Theme.surface
        border.width: 1
        border.color: Theme.border
    }

    contentItem: FocusScope {
        implicitHeight: column.implicitHeight
        focus: true

        Keys.onEscapePressed: {
            if (card.prompt)
                card.answer("deny")
            else
                card.close()
        }
        // Enter picks the prompt's primary choice; Ctrl+Enter hands the tab back.
        function pressEnter(event) {
            if (card.takenOver && (event.modifiers & Qt.ControlModifier)) {
                Agent.handBack(card.taskId)
            } else if (card.prompt) {
                const choices = card.prompt.choices
                card.answer(choices[choices.length - 1].id)
            }
        }
        Keys.onReturnPressed: event => pressEnter(event)
        Keys.onEnterPressed: event => pressEnter(event)

        ColumnLayout {
            id: column
            width: parent.width
            spacing: Theme.spacing * 2

            // Who, what was asked, and the controls.
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spacing * 2

                // Still while it waits for the person.
                AgentRing { working: card.working && !card.prompt && !card.takenOver }
                Text {
                    text: card.task ? card.task.agentName : ""
                    color: Theme.text
                    font.pixelSize: Theme.fontSize
                    font.bold: true
                }
                Text {
                    Layout.fillWidth: true
                    // The conversation's first question names it.
                    text: !card.task ? ""
                        : card.task.earlier.length > 0 ? card.task.earlier[0].question
                        : card.task.question
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSize
                    elide: Text.ElideRight
                }
                // Your hands always win: take the tab back at any moment.
                DialogButton {
                    visible: card.working && card.acting && !card.takenOver
                    implicitHeight: Theme.urlBarHeight - Theme.spacing
                    text: qsTr("Take over")
                    onClicked: Agent.takeOver(card.taskId, card.tabId)
                }
                DialogButton {
                    visible: card.working
                    implicitHeight: Theme.urlBarHeight - Theme.spacing
                    text: qsTr("Stop")
                    onClicked: Agent.stop(card.taskId)
                }
                IconButton {
                    implicitWidth: Theme.urlBarHeight - Theme.spacing
                    implicitHeight: implicitWidth
                    glyph: "✕"
                    tip: qsTr("Close (Esc)")
                    onClicked: card.close()
                }
            }

            // Everything below the header scrolls once the conversation
            // outgrows the window.
            Flickable {
                id: scroller
                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(body.implicitHeight, card.maxBodyHeight)
                contentWidth: width
                contentHeight: body.implicitHeight
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                interactive: contentHeight > height
                // Follow the newest step or answer.
                onContentHeightChanged: contentY = Math.max(0, contentHeight - height)

                ColumnLayout {
                    id: body
                    width: scroller.width
                    spacing: Theme.spacing * 2

                    // Earlier questions and their answers.
                    Repeater {
                        model: card.task ? card.task.earlier : []
                        delegate: ColumnLayout {
                            required property var modelData
                            required property int index
                            Layout.fillWidth: true
                            spacing: Theme.spacing

                            // The first question is in the header.
                            Text {
                                Layout.fillWidth: true
                                visible: index > 0
                                text: modelData.question
                                color: Theme.textMuted
                                font.pixelSize: Theme.fontSize
                                wrapMode: Text.Wrap
                            }
                            TextEdit {
                                Layout.fillWidth: true
                                visible: modelData.answer.length > 0
                                text: Agent.answerHtml(modelData.answer)
                                textFormat: TextEdit.RichText
                                readOnly: true
                                selectByMouse: true
                                wrapMode: TextEdit.Wrap
                                color: Theme.text
                                selectionColor: Theme.accent
                                selectedTextColor: Theme.onAccent
                                font.pixelSize: Theme.fontSize + 1
                            }
                            Text {
                                Layout.fillWidth: true
                                visible: modelData.answer.length === 0
                                text: modelData.status === "stopped" ? qsTr("Stopped.") : modelData.error
                                color: Theme.textMuted
                                font.pixelSize: Theme.fontSize
                                wrapMode: Text.Wrap
                            }
                        }
                    }

                    // The question being worked on, once there were others.
                    Text {
                        Layout.fillWidth: true
                        visible: card.task !== null && card.task.earlier.length > 0
                        text: card.task ? card.task.question : ""
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSize
                        wrapMode: Text.Wrap
                    }

                    // The log: what it did, in a few words each.
                    ColumnLayout {
                        Layout.fillWidth: true
                        visible: steps.count > 0 || (card.task !== null && card.task.status === "thinking")
                        spacing: Theme.spacing / 2

                        Repeater {
                            id: steps
                            model: card.task ? card.task.steps : []
                            delegate: RowLayout {
                                required property var modelData
                                Layout.fillWidth: true
                                spacing: Theme.spacing * 2

                                Item {
                                    implicitWidth: Theme.iconSize
                                    implicitHeight: Theme.fontSize + 4
                                    AgentRing {
                                        anchors.centerIn: parent
                                        visible: modelData.status === "running"
                                        size: Theme.fontSize - 2
                                    }
                                    Text {
                                        anchors.centerIn: parent
                                        visible: modelData.status !== "running"
                                        text: modelData.status === "done" ? "✓"
                                            : modelData.status === "denied" ? "⊘" : "✕"
                                        color: modelData.status === "done" ? Theme.textMuted
                                            : modelData.status === "denied" ? Theme.warning : Theme.danger
                                        font.pixelSize: Theme.fontSize - 1
                                    }
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: modelData.text
                                    color: modelData.status === "running" ? Theme.text : Theme.textMuted
                                    font.pixelSize: Theme.fontSize - 1
                                    elide: Text.ElideRight
                                }
                            }
                        }
                        // Between steps, while the model decides what's next.
                        RowLayout {
                            visible: card.task !== null && card.task.status === "thinking"
                            spacing: Theme.spacing * 2
                            // The header's ring already turns; one moving thing is enough.
                            Item {
                                implicitWidth: Theme.iconSize
                                implicitHeight: Theme.fontSize + 4
                            }
                            Text {
                                text: qsTr("Thinking")
                                color: Theme.textMuted
                                font.pixelSize: Theme.fontSize - 1
                            }
                        }
                    }

                    // The background tabs it opened, one click away.
                    Flow {
                        Layout.fillWidth: true
                        visible: card.task !== null && card.task.opened.length > 0
                        spacing: Theme.spacing

                        Repeater {
                            model: card.task ? card.task.opened : []
                            delegate: DialogButton {
                                required property var modelData
                                readonly property string title: modelData.title.length > 0 ? modelData.title : qsTr("Tab")
                                implicitHeight: Theme.urlBarHeight - Theme.spacing
                                text: title.length > 32 ? title.slice(0, 31) + "…" : title
                                onClicked: {
                                    const index = Tabs.indexOfTab(modelData.tab)
                                    if (index >= 0)
                                        Tabs.activate(index)
                                }
                            }
                        }
                    }

                    // An approval, asked right where the work happens.
                    Rectangle {
                        Layout.fillWidth: true
                        visible: card.prompt !== null
                        implicitHeight: promptColumn.implicitHeight + Theme.spacing * 4
                        radius: Theme.radius
                        color: Theme.surfaceRaised

                        ColumnLayout {
                            id: promptColumn
                            anchors.fill: parent
                            anchors.margins: Theme.spacing * 2
                            spacing: Theme.spacing * 2

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: Theme.spacing / 2
                                Text {
                                    Layout.fillWidth: true
                                    text: card.prompt ? card.prompt.text : ""
                                    color: Theme.text
                                    font.pixelSize: Theme.fontSize
                                    wrapMode: Text.Wrap
                                }
                                // Exactly what will happen.
                                Text {
                                    Layout.fillWidth: true
                                    visible: text.length > 0
                                    text: card.prompt ? card.prompt.detail : ""
                                    color: Theme.textMuted
                                    font.pixelSize: Theme.fontSize - 1
                                    wrapMode: Text.Wrap
                                }
                            }
                            RowLayout {
                                Layout.alignment: Qt.AlignRight
                                spacing: Theme.spacing

                                // Safety's choices, primary last.
                                Repeater {
                                    model: card.prompt ? card.prompt.choices : []
                                    delegate: DialogButton {
                                        required property var modelData
                                        required property int index
                                        readonly property bool last: index === card.prompt.choices.length - 1
                                        text: modelData.label + (last ? "  ⏎" : modelData.id === "deny" ? "  Esc" : "")
                                        primary: last && modelData.id !== "deny"
                                        enabled: modelData.id === "deny" || card.armed
                                        onClicked: card.answer(modelData.id)
                                    }
                                }
                            }
                        }
                    }

                    // The person has the tab; the agent waits for it back.
                    Rectangle {
                        Layout.fillWidth: true
                        visible: card.takenOver
                        implicitHeight: drivingRow.implicitHeight + Theme.spacing * 4
                        radius: Theme.radius
                        color: Theme.surfaceRaised

                        RowLayout {
                            id: drivingRow
                            anchors.fill: parent
                            anchors.margins: Theme.spacing * 2
                            spacing: Theme.spacing * 2

                            Text {
                                Layout.fillWidth: true
                                text: qsTr("You're driving. %1 waits until you hand the tab back.")
                                    .arg(card.task ? card.task.agentName : "")
                                color: Theme.text
                                font.pixelSize: Theme.fontSize
                                wrapMode: Text.Wrap
                            }
                            DialogButton {
                                text: qsTr("Hand back") + "  Ctrl ⏎"
                                primary: true
                                onClicked: Agent.handBack(card.taskId)
                            }
                        }
                    }

                    // The answer, short and selectable.
                    TextEdit {
                        Layout.fillWidth: true
                        // RichText always holds an HTML skeleton, so check the answer.
                        visible: card.task !== null && card.task.answer.length > 0
                        text: card.task ? Agent.answerHtml(card.task.answer) : ""
                        textFormat: TextEdit.RichText
                        readOnly: true
                        selectByMouse: true
                        wrapMode: TextEdit.Wrap
                        color: Theme.text
                        selectionColor: Theme.accent
                        selectedTextColor: Theme.onAccent
                        font.pixelSize: Theme.fontSize + 1
                    }

                    Text {
                        Layout.fillWidth: true
                        visible: text.length > 0
                        text: !card.task ? ""
                            : card.task.status === "failed" ? card.task.error
                            : card.task.status === "stopped" ? qsTr("Stopped.") : ""
                        color: card.task && card.task.status === "failed" ? Theme.danger : Theme.textMuted
                        font.pixelSize: Theme.fontSize
                        wrapMode: Text.Wrap
                    }
                }
            }

            // Keep talking once it has answered.
            IonTextField {
                id: followUp
                Layout.fillWidth: true
                visible: card.task !== null && !card.working
                placeholderText: qsTr("Ask %1 a follow-up").arg(card.task ? card.task.agentName : "")
                onAccepted: {
                    const question = text.trim()
                    if (question.length > 0 && Agent.followUp(card.taskId, question, card.pageUrl, card.pageTitle))
                        text = ""
                }
            }
        }
    }
}
