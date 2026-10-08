import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Beside the page: where an agent task goes once it becomes a conversation
// (a follow-up, or the card's move button), and where the person can start
// one with `shortcuts.agentSidebar`. While it is open, new questions from
// the address bar land here too.
Rectangle {
    id: sidebar

    // The current tab's view, for follow-ups and new questions.
    property var view: null
    property bool shown: false
    // The task shown, or -1 for a fresh conversation.
    property int taskId: -1
    readonly property bool hasTask: {
        Agent.revision
        return taskId >= 0 && Agent.taskJson(taskId).length > 0
    }

    // A new question typed here; the address bar runs it like its own.
    signal ask(string input)
    // Closed, so the page can take focus back.
    signal finished()
    // The person wants to see what agents did and may do.
    signal activityRequested()

    // Show `task` here, letting go of a finished one it replaces.
    function show(task) {
        if (taskId >= 0 && taskId !== task)
            Agent.dismiss(taskId)
        taskId = task
        shown = true
        Qt.callLater(focusInput)
    }
    function hide() {
        shown = false
        finished()
    }
    function toggle(task) {
        if (shown)
            hide()
        else
            show(task >= 0 ? task : taskId)
    }
    function startOver() {
        if (taskId >= 0)
            Agent.dismiss(taskId)
        taskId = -1
        Qt.callLater(focusInput)
    }
    function focusInput() {
        if (hasTask) {
            conversation.forceActiveFocus()
            conversation.focusInput()
        } else {
            question.forceActiveFocus()
        }
    }

    visible: shown
    implicitWidth: Theme.agentSidebarWidth
    color: Theme.surface

    // Hairline between the page and the sidebar.
    Rectangle {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: Theme.hairline
        color: Theme.border
    }

    AgentConversation {
        id: conversation
        anchors.fill: parent
        anchors.margins: Theme.spacing * 3
        visible: sidebar.hasTask
        fill: true
        renewable: true
        taskId: sidebar.hasTask ? sidebar.taskId : -1
        tabId: sidebar.view ? sidebar.view.tabId : -1
        pageUrl: sidebar.view ? sidebar.view.url.toString() : ""
        pageTitle: sidebar.view ? sidebar.view.title : ""
        onCloseRequested: sidebar.hide()
        onNewRequested: sidebar.startOver()
    }

    // Nothing asked yet: say what it's for, and take the question.
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.spacing * 3
        visible: !sidebar.hasTask
        spacing: Theme.spacing * 2

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing * 2

            AgentRing {}
            Text {
                Layout.fillWidth: true
                text: qsTr("Ion Agent")
                color: Theme.text
                font.pixelSize: Theme.fontSize
                font.bold: true
            }
            IconButton {
                implicitWidth: Theme.urlBarHeight - Theme.spacing
                implicitHeight: implicitWidth
                glyph: "◷"
                tip: qsTr("Agent activity")
                onClicked: sidebar.activityRequested()
            }
            IconButton {
                implicitWidth: Theme.urlBarHeight - Theme.spacing
                implicitHeight: implicitWidth
                glyph: "✕"
                tip: qsTr("Close")
                onClicked: sidebar.hide()
            }
        }
        Item { Layout.fillHeight: true }
        Text {
            Layout.fillWidth: true
            text: qsTr("Ask about this page, or have it do something here. Start with @name to ask one of your own agents.")
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize
            wrapMode: Text.Wrap
        }
        IonTextField {
            id: question
            Layout.fillWidth: true
            placeholderText: qsTr("Ask Ion Agent")
            Keys.onEscapePressed: sidebar.hide()
            onAccepted: {
                const input = text.trim()
                if (input.length === 0)
                    return
                text = ""
                // Ion Agent unless another is named.
                sidebar.ask(/^(@\S+|!ai)\s/.test(input) ? input : "@ion " + input)
            }
        }
    }
}
