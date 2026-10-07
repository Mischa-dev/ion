import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// The current workspace's icon or color dot, plus its name once there is more than one
// workspace, in either tab strip. Clicking it asks for the workspace menu.
ToolButton {
    id: control

    // Only the dot, even with several workspaces.
    property bool compact: false
    readonly property bool showName: !compact && count > 1
    // The current workspace as `{id, name, color, icon, tabs}`, and how many there are.
    property var current: ({ name: "", color: "" })
    property int count: 1

    signal menuRequested(Item anchor)

    function refresh() {
        const list = JSON.parse(Tabs.workspaces())
        count = list.length
        current = list.find(w => w.id === Tabs.workspace) ?? list[0] ?? { name: "", color: "" }
    }

    implicitHeight: Theme.urlBarHeight
    leftPadding: Theme.spacing * 2
    rightPadding: Theme.spacing * 2
    focusPolicy: Qt.NoFocus

    Component.onCompleted: refresh()
    Connections {
        target: Tabs
        function onWorkspaceChanged() { control.refresh() }
        function onWorkspacesChanged() { control.refresh() }
    }

    // A click hides the tip until the pointer leaves, so it doesn't cover the
    // menu the button just opened.
    property bool tipDismissed: false
    onHoveredChanged: if (!hovered) tipDismissed = false
    onClicked: {
        tipDismissed = true
        control.menuRequested(control)
    }

    contentItem: RowLayout {
        spacing: Theme.spacing

        Text {
            Layout.alignment: Qt.AlignVCenter
            // Long icons shrink, then elide, in narrow strips such as the
            // collapsed sidebar.
            Layout.maximumWidth: control.availableWidth
            visible: (control.current.icon ?? "").length > 0
            text: control.current.icon ?? ""
            color: Theme.text
            font.pixelSize: Theme.fontSize
            fontSizeMode: Text.HorizontalFit
            minimumPixelSize: Theme.fontSizeMin
            elide: Text.ElideRight
            textFormat: Text.PlainText
        }
        Rectangle {
            Layout.alignment: Qt.AlignVCenter
            visible: (control.current.icon ?? "").length === 0
            implicitWidth: Theme.workspaceDotSize
            implicitHeight: Theme.workspaceDotSize
            radius: width / 2
            color: control.current.color.length > 0 ? control.current.color : Theme.accent
        }
        Text {
            Layout.fillWidth: true
            visible: control.showName
            text: control.current.name
            textFormat: Text.PlainText
            color: Theme.text
            font.pixelSize: Theme.fontSize
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
        }
    }

    background: Rectangle {
        radius: Theme.radius
        color: control.down ? Theme.surfaceRaised : control.hovered ? Theme.surfaceHover : "transparent"
    }

    ToolTip.visible: hovered && !tipDismissed && !showName
    ToolTip.text: qsTr("Workspace: %1").arg(current.name)
    ToolTip.delay: 600
}
