import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// In the toolbar while an agent works on the current tab with its card put
// away: the agent's ring and name. Clicking brings the card back.
ToolButton {
    id: chip

    property int taskId: -1
    readonly property var task: {
        Agent.revision
        const json = taskId >= 0 ? Agent.taskJson(taskId) : ""
        return json.length > 0 ? JSON.parse(json) : null
    }
    readonly property bool working: task !== null
        && (task.status === "thinking" || task.status === "working")
    readonly property bool waiting: task !== null && task.prompt !== null

    implicitHeight: Theme.urlBarHeight
    leftPadding: Theme.spacing * 2
    rightPadding: Theme.spacing * 2
    focusPolicy: Qt.NoFocus

    contentItem: RowLayout {
        spacing: Theme.spacing * 1.5
        AgentRing { working: chip.working && !chip.waiting }
        Text {
            text: chip.task ? (chip.waiting ? qsTr("%1 needs you").arg(chip.task.agentName) : chip.task.agentName) : ""
            color: chip.waiting ? Theme.accent : Theme.text
            font.pixelSize: Theme.fontSize
        }
    }

    background: Rectangle {
        radius: Theme.radius
        color: chip.down ? Theme.surfaceRaised : chip.hovered ? Theme.surfaceHover : "transparent"
    }

    ToolTip.visible: hovered
    ToolTip.text: qsTr("Show what %1 is doing").arg(task ? task.agentName : "")
    ToolTip.delay: 600
}
