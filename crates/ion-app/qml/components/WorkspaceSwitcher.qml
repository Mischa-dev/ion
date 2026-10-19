import QtQuick
import QtQuick.Controls
import Ion

// One dot (or icon) per workspace at the foot of the sidebar, like Arc's
// spaces: click one to switch to it, or drop a dragged tab on it to move the
// tab there. Hidden while there is only one workspace.
Row {
    id: switcher

    // `{id, name, color, icon, tabs}` per workspace.
    property var workspaces: []
    // The workspace a dragged tab is over, or -1.
    property int dropId: -1

    function refresh() { workspaces = JSON.parse(Tabs.workspaces()) }

    // The workspace whose dot is at `scenePos` (window coordinates), or -1.
    function idAt(scenePos) {
        if (!visible)
            return -1
        const p = mapFromItem(null, scenePos.x, scenePos.y)
        const item = childAt(p.x, p.y)
        return item && item.workspaceId !== undefined ? item.workspaceId : -1
    }

    visible: workspaces.length > 1
    spacing: 0

    Component.onCompleted: refresh()
    Connections {
        target: Tabs
        function onWorkspaceChanged() { switcher.refresh() }
        function onWorkspacesChanged() { switcher.refresh() }
        function onSessionChanged() { switcher.refresh() }
    }

    Repeater {
        model: switcher.workspaces

        ToolButton {
            id: dot

            required property var modelData
            readonly property int workspaceId: modelData.id
            readonly property bool current: modelData.id === Tabs.workspace
            readonly property bool dropping: modelData.id === switcher.dropId

            implicitWidth: Theme.urlBarHeight * 3 / 4
            implicitHeight: Theme.urlBarHeight
            focusPolicy: Qt.NoFocus
            onClicked: Tabs.switchWorkspace(modelData.id)

            ToolTip.visible: hovered
            ToolTip.delay: Theme.hoverDelayMs
            ToolTip.text: modelData.tabs === 1 ? qsTr("%1 · 1 tab").arg(modelData.name)
                                               : qsTr("%1 · %2 tabs").arg(modelData.name).arg(modelData.tabs)

            background: Rectangle {
                radius: Theme.radius
                color: dot.dropping ? Theme.surfaceHover : dot.hovered ? Theme.surfaceRaised : "transparent"
                border.width: dot.dropping ? Theme.groupLineWidth : 0
                border.color: Theme.accent
            }

            contentItem: Item {
                Text {
                    anchors.centerIn: parent
                    visible: (dot.modelData.icon ?? "").length > 0
                    text: dot.modelData.icon ?? ""
                    color: Theme.text
                    opacity: dot.current ? 1 : Theme.dimmedOpacity
                    font.pixelSize: Theme.fontSize
                    textFormat: Text.PlainText
                }
                Rectangle {
                    anchors.centerIn: parent
                    visible: (dot.modelData.icon ?? "").length === 0
                    // The current workspace's dot is a little larger.
                    width: dot.current ? Theme.workspaceDotSize : Theme.workspaceDotSize * 0.7
                    height: width
                    radius: width / 2
                    color: dot.modelData.color.length > 0 ? dot.modelData.color : Theme.accent
                    opacity: dot.current || dot.hovered ? 1 : Theme.dimmedOpacity

                    Behavior on width { NumberAnimation { duration: Theme.animationMs } }
                }
            }
        }
    }
}
