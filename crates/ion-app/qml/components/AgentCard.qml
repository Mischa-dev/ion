import QtQuick
import QtQuick.Controls
import Ion

// What an agent is doing for the current tab, dropped from the address bar.
// Opened when the person asks with `@ion …` or `!ai …`; Esc or a click
// elsewhere puts it away (the address bar's agent chip brings it back while
// the agent works). Once it becomes a conversation it moves to the agent
// sidebar.
Popup {
    id: card

    // The `Agent` task shown, or -1.
    property alias taskId: conversation.taskId
    // The tab the person is looking at, which Take over takes back, and
    // what it shows, for follow-ups.
    property alias tabId: conversation.tabId
    property alias pageUrl: conversation.pageUrl
    property alias pageTitle: conversation.pageTitle
    // How tall the conversation gets before it scrolls.
    property alias maxBodyHeight: conversation.maxBodyHeight
    // Emitted when the card closes, so the page can take focus back.
    signal finished()
    // The person asked a follow-up or moved it: show `task` in the sidebar.
    signal moveToSide(int task)

    // Set while handing the task to the sidebar, so closing keeps it.
    property bool moving: false

    function openFor(id) {
        taskId = id
        open()
    }

    function moveAway() {
        const task = taskId
        moving = true
        close()
        moving = false
        moveToSide(task)
    }

    onOpened: conversation.focusInput()

    // A finished task is forgotten once put away; a working one keeps going
    // behind the chip.
    onClosed: {
        if (!moving && conversation.task && !conversation.working)
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

    contentItem: AgentConversation {
        id: conversation
        focus: true
        movable: true
        onCloseRequested: card.close()
        onMoveRequested: card.moveAway()
        // Asking again makes it a conversation, which belongs beside the page.
        onFollowedUp: card.moveAway()
    }
}
