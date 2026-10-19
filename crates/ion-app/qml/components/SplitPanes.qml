import QtQuick
import Ion

// Lays out a split view: while the current tab is in one, its tabs' web views
// share the page area side by side (or stacked), with handles between them to
// resize. Fills the page area; Main.qml places each view at rectFor(row).
//
// Also shows where a tab dragged out of the strip would land (showDrop).
Item {
    id: root

    // The split on screen as `{id, tabs, sizes, vertical}`, or null.
    property var split: null
    // Pane sizes as shown: the split's, or live ones while a handle is dragged.
    property var sizes: []
    readonly property bool active: split !== null
    readonly property real gap: Theme.spacing
    // A tab being dragged over the page: which half would get it, or "".
    property string dropSide: ""

    // The JSON `split` came from, so a refresh that changes nothing keeps
    // live sizes during a handle drag.
    property string splitJson: ""

    function refresh() {
        const json = Tabs.currentSplit()
        if (json === splitJson)
            return
        splitJson = json
        const next = JSON.parse(json)
        split = next
        sizes = next ? next.sizes : []
    }
    Component.onCompleted: refresh()
    Connections {
        target: Tabs
        function onSplitsChanged() { root.refresh() }
        function onCurrentIndexChanged() { root.refresh() }
        function onCountChanged() { root.refresh() }
        // Rows moved: the split's tabs may sit at other rows now.
        function onSessionChanged() { root.refresh() }
    }

    // Position of the tab at `row` among the panes, or -1.
    function paneOf(row) {
        return split ? split.tabs.indexOf(row) : -1
    }

    // Where the view of the tab at `row` goes; the whole area unless it is a
    // pane. `w`, `h` and `s` are passed so bindings follow them.
    function rectFor(row, w, h, s) {
        const pane = paneOf(row)
        if (pane < 0 || s.length !== split.tabs.length)
            return Qt.rect(0, 0, w, h)
        const length = (split.vertical ? h : w) - gap * (s.length - 1)
        let start = 0
        for (let i = 0; i < pane; i++)
            start += s[i] * length + gap
        const size = s[pane] * length
        return split.vertical ? Qt.rect(0, Math.round(start), w, Math.round(size))
                              : Qt.rect(Math.round(start), 0, Math.round(size), h)
    }

    // Show where a tab dragged from the strip would land; `point` is in this
    // item's coordinates, or null to hide the hint.
    function showDrop(point) {
        if (!point || point.x < 0 || point.y < 0 || point.x > width || point.y > height)
            dropSide = ""
        else
            dropSide = point.x < width / 2 ? "left" : "right"
    }

    // The current pane's outline, so it's clear which one the toolbar acts on.
    Rectangle {
        readonly property rect pane: root.rectFor(Tabs.currentIndex, root.width, root.height, root.sizes)
        visible: root.active
        x: pane.x
        y: pane.y
        width: pane.width
        height: pane.height
        z: 2
        color: "transparent"
        radius: Theme.radius
        border.width: Theme.groupLineWidth
        border.color: Theme.accent
    }

    // Handles between neighbouring panes.
    Repeater {
        model: root.active ? root.split.tabs.length - 1 : 0
        delegate: MouseArea {
            id: handle
            required property int index
            readonly property rect before: root.rectFor(root.split.tabs[index], root.width, root.height, root.sizes)
            readonly property bool vertical: root.split.vertical

            z: 3
            x: vertical ? 0 : before.x + before.width
            y: vertical ? before.y + before.height : 0
            width: vertical ? root.width : root.gap
            height: vertical ? root.gap : root.height
            hoverEnabled: true
            cursorShape: vertical ? Qt.SplitVCursor : Qt.SplitHCursor
            preventStealing: true

            Rectangle {
                anchors.centerIn: parent
                width: handle.vertical ? Theme.tabHeight : Theme.groupLineWidth
                height: handle.vertical ? Theme.groupLineWidth : Theme.tabHeight
                radius: Theme.groupLineWidth
                color: handle.containsMouse || handle.pressed ? Theme.accent : Theme.border
            }

            onPositionChanged: mouse => {
                if (!pressed)
                    return
                const p = mapToItem(root, mouse.x, mouse.y)
                const s = root.sizes.slice()
                const total = (vertical ? root.height : root.width) - root.gap * (s.length - 1)
                // The two panes either side trade space; the others keep theirs.
                let start = 0
                for (let i = 0; i < index; i++)
                    start += s[i] * total + root.gap
                const pair = s[index] + s[index + 1]
                const min = 0.1
                const first = Math.max(min, Math.min(pair - min, ((vertical ? p.y : p.x) - start) / total))
                s[index] = first
                s[index + 1] = pair - first
                root.sizes = s
            }
            onReleased: Tabs.setSplitSizes(root.split.id, root.sizes)
            // Double-click evens the two panes out.
            onDoubleClicked: {
                const s = root.sizes.slice()
                const pair = s[index] + s[index + 1]
                s[index] = s[index + 1] = pair / 2
                Tabs.setSplitSizes(root.split.id, s)
            }
        }
    }

    // A tab dragged over the page: the half it would take.
    Rectangle {
        visible: root.dropSide.length > 0
        z: 4
        x: root.dropSide === "left" ? 0 : root.width / 2
        width: root.width / 2
        height: root.height
        radius: Theme.radius
        color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, Theme.tintOpacity)
        border.width: Theme.groupLineWidth
        border.color: Theme.accent

        // On a chip, so it reads over any page.
        Rectangle {
            anchors.centerIn: parent
            width: hint.implicitWidth + Theme.spacing * 4
            height: hint.implicitHeight + Theme.spacing * 2
            radius: height / 2
            color: Theme.surface
            border.color: Theme.border
            border.width: 1

            Text {
                id: hint
                anchors.centerIn: parent
                text: qsTr("Open in split view")
                color: Theme.text
                font.pixelSize: Theme.fontSize + 2
                font.bold: true
            }
        }
    }
}
