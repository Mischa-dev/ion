import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Beside the page: where an agent task goes once it becomes a conversation
// (a follow-up, or the card's move button), and where the person can start
// one with `shortcuts.agentSidebar`. While it is open, new questions from
// the address bar land here too. With nothing open it lists earlier
// conversations (kept for `ai.keepConversationsDays`) to carry on.
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
    // Saved conversations, while the empty state shows them.
    readonly property var conversations: {
        Agent.revision
        return shown && !hasTask ? JSON.parse(Agent.conversationsJson() || "[]") : []
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
    // Carry on a saved conversation here.
    function reopen(key) {
        const task = Agent.reopen(key, view ? view.tabId : -1)
        if (task >= 0)
            show(task)
    }
    // "5 min ago", "Yesterday", "3 Oct".
    function when(time) {
        const ago = Date.now() / 1000 - time
        if (ago < 60)
            return qsTr("Just now")
        if (ago < 3600)
            return qsTr("%1 min ago").arg(Math.floor(ago / 60))
        const date = new Date(time * 1000)
        const today = new Date()
        const start = d => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()
        const days = Math.round((start(today) - start(date)) / 86400000)
        if (days === 0)
            return date.toLocaleTimeString(Qt.locale(), Locale.ShortFormat)
        if (days === 1)
            return qsTr("Yesterday")
        return date.toLocaleDateString(Qt.locale(), "d MMM")
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
    // Dragging the left edge resizes it, up to most of the window.
    property real chosenWidth: Theme.agentSidebarWidth
    implicitWidth: chosenWidth
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
        // Earlier conversations, newest first.
        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: Theme.spacing
            visible: sidebar.conversations.length > 0
            Text {
                Layout.fillWidth: true
                text: qsTr("Earlier")
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize - 1
                font.bold: true
            }
            Text {
                id: forgetAll
                // The first click asks again.
                property bool armed: false
                text: armed ? qsTr("Forget all?") : qsTr("Forget all")
                color: armed ? Theme.danger : (forgetAllArea.containsMouse ? Theme.text : Theme.textMuted)
                font.pixelSize: Theme.fontSize - 1
                MouseArea {
                    id: forgetAllArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onExited: forgetAll.armed = false
                    onClicked: {
                        if (forgetAll.armed)
                            Agent.forgetConversations()
                        forgetAll.armed = !forgetAll.armed
                    }
                }
            }
        }
        ListView {
            id: earlier
            Layout.fillWidth: true
            Layout.fillHeight: true
            // Rows line up with the heading; their hover reaches past it.
            Layout.leftMargin: -Theme.spacing * 2
            Layout.rightMargin: -Theme.spacing
            clip: true
            spacing: Theme.hairline
            boundsBehavior: Flickable.StopAtBounds
            model: sidebar.conversations
            delegate: Rectangle {
                id: row
                required property var modelData
                width: earlier.width
                implicitHeight: rowText.implicitHeight + Theme.spacing * 2
                radius: Theme.radius
                color: rowArea.containsMouse ? Theme.surfaceHover : "transparent"

                MouseArea {
                    id: rowArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: sidebar.reopen(row.modelData.key)
                }
                ColumnLayout {
                    id: rowText
                    anchors.left: parent.left
                    anchors.right: forget.left
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: Theme.spacing * 2
                    spacing: 0
                    Text {
                        Layout.fillWidth: true
                        text: row.modelData.question
                        color: Theme.text
                        font.pixelSize: Theme.fontSize
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        text: {
                            const parts = []
                            // Ion Agent goes without saying.
                            if (row.modelData.agent !== "ion")
                                parts.push(row.modelData.agentName)
                            if (row.modelData.site)
                                parts.push(row.modelData.site)
                            parts.push(sidebar.when(row.modelData.updated))
                            if (row.modelData.count > 1)
                                parts.push(qsTr("%1 questions").arg(row.modelData.count))
                            return parts.join(" · ")
                        }
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSize - 1
                        elide: Text.ElideRight
                    }
                }
                IconButton {
                    id: forget
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    implicitWidth: Theme.urlBarHeight - Theme.spacing
                    implicitHeight: implicitWidth
                    opacity: rowArea.containsMouse || hovered ? 1 : 0
                    glyph: "✕"
                    tip: qsTr("Forget this conversation")
                    onClicked: Agent.forgetConversation(row.modelData.key)
                }
            }
        }
        Item {
            Layout.fillHeight: true
            visible: sidebar.conversations.length === 0
        }
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

    MouseArea {
        id: resizer
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: Theme.spacing
        cursorShape: Qt.SizeHorCursor
        property real startX: 0
        property real startWidth: 0
        onPressed: mouse => {
            startX = mapToItem(null, mouse.x, 0).x
            startWidth = sidebar.chosenWidth
        }
        onPositionChanged: mouse => {
            if (!pressed)
                return
            const most = sidebar.parent ? sidebar.parent.width * 0.6 : startWidth
            const x = mapToItem(null, mouse.x, 0).x
            sidebar.chosenWidth = Math.max(Theme.agentSidebarMinWidth,
                Math.min(most, startWidth + startX - x))
        }
    }
}
