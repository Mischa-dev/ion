import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// One agent task as a conversation: who is working, what was asked, a
// short log of steps, any approval it needs, the answers, and a field for
// the next question. Shown in the card under the address bar and, once it
// becomes a conversation, in the agent sidebar.
FocusScope {
    id: conversation

    // The `Agent` task shown, or -1.
    property int taskId: -1
    // The tab the person is looking at, and what it shows, for follow-ups.
    property int tabId: -1
    property string pageUrl: ""
    property string pageTitle: ""
    // How tall the conversation gets before it scrolls. Ignored when
    // `fill` is set: then it takes whatever height it is given.
    property real maxBodyHeight: 480
    property bool fill: false
    // Shown in the card: offer to move it to the sidebar.
    property bool movable: false
    // Shown in the sidebar: offer to start over once it has answered.
    property bool renewable: false
    readonly property var task: {
        Agent.revision
        const json = taskId >= 0 ? Agent.taskJson(taskId) : ""
        return json.length > 0 ? JSON.parse(json) : null
    }
    readonly property bool working: task !== null
        && (task.status === "thinking" || task.status === "working")
    readonly property var prompt: task ? task.prompt : null
    // Waiting on the model with nothing written yet.
    readonly property bool thinking: task !== null && task.status === "thinking" && task.draft.length === 0
    // The agent has been changing the page, and whether the person took the
    // tab back from it.
    readonly property bool acting: task !== null && task.acting
    readonly property bool takenOver: task !== null && task.takenOver
    // The tab Take over takes: the one in view when it is the task's,
    // otherwise the one the task was asked from.
    readonly property int takeOverTab: {
        if (!task)
            return -1
        if (tabId === task.tab || task.opened.some(t => t.tab === tabId))
            return tabId
        return task.tab
    }

    // Esc with nothing to deny, the close button, and a new follow-up.
    signal closeRequested()
    signal moveRequested()
    signal newRequested()
    signal followedUp()

    implicitHeight: column.implicitHeight

    // Typing goes to the follow-up field, except while an approval waits:
    // then Enter and Esc answer it.
    function focusInput() {
        if (task && !prompt)
            followUp.forceActiveFocus()
    }

    // Allow stays disabled for a moment after a prompt appears, so a click or
    // key already on its way can't land on it (docs/SAFETY.md).
    property bool armed: false
    onPromptChanged: {
        armed = false
        if (prompt) {
            armTimer.restart()
            // Out of the follow-up field, so Enter there can't send and Esc
            // reaches the prompt. What was typed stays.
            if (followUp.activeFocus) {
                followUp.focus = false
                conversation.forceActiveFocus()
            }
        } else if (activeFocus) {
            focusInput()
        }
    }
    Timer {
        id: armTimer
        interval: Theme.promptArmMs
        onTriggered: conversation.armed = true
    }

    function answer(choice) {
        if (!prompt || (choice !== "deny" && !armed))
            return
        Agent.answer(taskId, choice)
    }

    // Typing goes to the follow-up field whenever no approval waits.
    onWorkingChanged: if (visible && activeFocus) focusInput()

    Keys.onEscapePressed: {
        if (conversation.prompt)
            conversation.answer("deny")
        else
            conversation.closeRequested()
    }
    // Enter picks the prompt's primary choice; Ctrl+Enter hands the tab back.
    function pressEnter(event) {
        if (conversation.takenOver && (event.modifiers & Qt.ControlModifier)) {
            Agent.handBack(conversation.taskId)
        } else if (conversation.prompt) {
            const choices = conversation.prompt.choices
            conversation.answer(choices[choices.length - 1].id)
        }
    }
    Keys.onReturnPressed: event => pressEnter(event)
    Keys.onEnterPressed: event => pressEnter(event)

    // What the person asked, set apart from the answers.
    component Question: Rectangle {
        property alias text: label.text
        Layout.fillWidth: true
        implicitHeight: label.implicitHeight + Theme.spacing * 2
        radius: Theme.radius
        color: Theme.surfaceRaised
        Text {
            id: label
            anchors.fill: parent
            anchors.margins: Theme.spacing
            anchors.leftMargin: Theme.spacing * 2
            anchors.rightMargin: Theme.spacing * 2
            color: Theme.text
            font.pixelSize: Theme.fontSize
            wrapMode: Text.Wrap
        }
    }

    ColumnLayout {
        id: column
        width: parent.width
        height: conversation.fill ? parent.height : implicitHeight
        spacing: Theme.spacing * 2

        // Who, what was asked, and the controls.
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing * 2

            // Still while it waits for the person.
            AgentRing { working: conversation.working && !conversation.prompt && !conversation.takenOver }
            Text {
                text: conversation.task ? conversation.task.agentName : ""
                color: Theme.text
                font.pixelSize: Theme.fontSize
                font.bold: true
            }
            Text {
                Layout.fillWidth: true
                // The conversation's first question names it. The sidebar
                // has room to show every question in place instead.
                visible: !conversation.fill
                text: !conversation.task ? ""
                    : conversation.task.earlier.length > 0 ? conversation.task.earlier[0].question
                    : conversation.task.question
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize
                elide: Text.ElideRight
            }
            // Your hands always win: take the tab back at any moment.
            DialogButton {
                visible: conversation.working && conversation.acting && !conversation.takenOver
                implicitHeight: Theme.urlBarHeight - Theme.spacing
                text: qsTr("Take over")
                onClicked: Agent.takeOver(conversation.taskId, conversation.takeOverTab)
            }
            DialogButton {
                visible: conversation.working
                implicitHeight: Theme.urlBarHeight - Theme.spacing
                text: qsTr("Stop")
                onClicked: Agent.stop(conversation.taskId)
            }
            Item {
                visible: conversation.fill
                Layout.fillWidth: true
            }
            IconButton {
                visible: conversation.renewable && !conversation.working
                implicitWidth: Theme.urlBarHeight - Theme.spacing
                implicitHeight: implicitWidth
                glyph: "+"
                tip: qsTr("New conversation")
                onClicked: conversation.newRequested()
            }
            // A conversation reads better beside the page than over it.
            IconButton {
                visible: conversation.movable
                implicitWidth: Theme.urlBarHeight - Theme.spacing
                implicitHeight: implicitWidth
                glyph: "◨"
                tip: qsTr("Move to the side")
                onClicked: conversation.moveRequested()
            }
            IconButton {
                implicitWidth: Theme.urlBarHeight - Theme.spacing
                implicitHeight: implicitWidth
                glyph: "✕"
                tip: qsTr("Close")
                onClicked: conversation.closeRequested()
            }
        }

        // Everything below the header scrolls once the conversation
        // outgrows the window.
        Flickable {
            id: scroller
            Layout.fillWidth: true
            Layout.preferredHeight: conversation.fill ? -1
                : Math.min(body.implicitHeight, conversation.maxBodyHeight)
            Layout.fillHeight: conversation.fill
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
                    model: conversation.task ? conversation.task.earlier : []
                    delegate: ColumnLayout {
                        required property var modelData
                        required property int index
                        Layout.fillWidth: true
                        spacing: Theme.spacing

                        // The card's header already shows the first question.
                        Question {
                            visible: index > 0 || conversation.fill
                            text: modelData.question
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
                Question {
                    visible: conversation.task !== null
                        && (conversation.task.earlier.length > 0 || conversation.fill)
                    text: conversation.task ? conversation.task.question : ""
                }

                // The log: what it did, in a few words each.
                ColumnLayout {
                    Layout.fillWidth: true
                    visible: steps.count > 0 || conversation.thinking
                    spacing: Theme.spacing / 2

                    Repeater {
                        id: steps
                        model: conversation.task ? conversation.task.steps : []
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
                        visible: conversation.thinking
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
                    visible: conversation.task !== null && conversation.task.opened.length > 0
                    spacing: Theme.spacing

                    Repeater {
                        model: conversation.task ? conversation.task.opened : []
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
                    visible: conversation.prompt !== null
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
                                text: conversation.prompt ? conversation.prompt.text : ""
                                color: Theme.text
                                font.pixelSize: Theme.fontSize
                                wrapMode: Text.Wrap
                            }
                            // Exactly what will happen.
                            Text {
                                Layout.fillWidth: true
                                visible: text.length > 0
                                text: conversation.prompt ? conversation.prompt.detail : ""
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
                                model: conversation.prompt ? conversation.prompt.choices : []
                                delegate: DialogButton {
                                    required property var modelData
                                    required property int index
                                    readonly property bool last: index === conversation.prompt.choices.length - 1
                                    text: modelData.label + (last ? "  ⏎" : modelData.id === "deny" ? "  Esc" : "")
                                    primary: last && modelData.id !== "deny"
                                    enabled: modelData.id === "deny" || conversation.armed
                                    onClicked: conversation.answer(modelData.id)
                                }
                            }
                        }
                    }
                }

                // The person has the tab; the agent waits for it back.
                Rectangle {
                    Layout.fillWidth: true
                    visible: conversation.takenOver
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
                                .arg(conversation.task ? conversation.task.agentName : "")
                            color: Theme.text
                            font.pixelSize: Theme.fontSize
                            wrapMode: Text.Wrap
                        }
                        DialogButton {
                            text: qsTr("Hand back") + "  Ctrl ⏎"
                            primary: true
                            onClicked: Agent.handBack(conversation.taskId)
                        }
                    }
                }

                // The answer, short and selectable.
                TextEdit {
                    Layout.fillWidth: true
                    // RichText always holds an HTML skeleton, so check the answer.
                    // While the model writes, its draft shows here.
                    readonly property string markdown: !conversation.task ? ""
                        : conversation.task.answer.length > 0 ? conversation.task.answer : conversation.task.draft
                    visible: markdown.length > 0
                    text: Agent.answerHtml(markdown)
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
                    text: !conversation.task ? ""
                        : conversation.task.status === "failed" ? conversation.task.error
                        : conversation.task.status === "stopped" ? qsTr("Stopped.") : ""
                    color: conversation.task && conversation.task.status === "failed" ? Theme.danger : Theme.textMuted
                    font.pixelSize: Theme.fontSize
                    wrapMode: Text.Wrap
                }
            }
        }

        // Keep talking. The next question can be typed while it works; it
        // is sent with Enter once it has answered.
        IonTextField {
            id: followUp
            Layout.fillWidth: true
            visible: conversation.task !== null
            placeholderText: qsTr("Ask %1 a follow-up").arg(conversation.task ? conversation.task.agentName : "")
            // Ctrl+Enter still hands a taken-back tab over.
            Keys.onReturnPressed: event => {
                if (conversation.takenOver && (event.modifiers & Qt.ControlModifier))
                    conversation.pressEnter(event)
                else
                    event.accepted = false
            }
            onAccepted: {
                if (conversation.working)
                    return
                const question = text.trim()
                if (question.length > 0 && Agent.followUp(conversation.taskId, question, conversation.pageUrl, conversation.pageTitle)) {
                    text = ""
                    conversation.followedUp()
                }
            }
        }
    }
}
