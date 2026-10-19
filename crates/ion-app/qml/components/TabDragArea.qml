import QtQuick
import Ion

// Mouse handling for one tab in TabStrip or VerticalTabStrip: left press
// activates the tab, middle click closes it, right click opens its menu, and
// dragging moves it along the strip. While dragging, the tab follows the
// pointer and a line marks where it will land; letting go moves it there in
// one step (joining or leaving a tab group by where it lands). Rows are not
// moved during the drag: the strips' filtered models recreate moved rows,
// which would end the drag. Dragged off the strip, the tab follows the
// pointer freely; let go over the page, it opens in split view.
MouseArea {
    id: area

    required property var list        // the strip's ListView or GridView; delegates have `row`
    required property int index       // the tab's row in the model

    readonly property bool dragging: dragActive
    property bool dragActive: false
    property point pressPos
    // How far the tab has been dragged, along the list only; the tab applies
    // it as a translation.
    readonly property real dragX: dragActive && (outside || !vertical) ? dragPos.x - pressPos.x : 0
    readonly property real dragY: dragActive && (outside || vertical) ? dragPos.y - pressPos.y : 0
    property point dragPos
    // The tab the dragged one would take the place of, or null.
    property var target: null
    // The pointer has left the strip during the drag.
    property bool outside: false

    readonly property bool vertical: list.orientation === ListView.Vertical

    signal activated(int index)
    signal closeRequested(int index)
    signal moveRequested(int from, int to)
    signal menuRequested(int index)
    // The tab is dragged off the strip, or let go there, at `scenePos` in
    // window coordinates. draggedOut(-1, …) means it came back or the drag
    // ended.
    signal draggedOut(int index, point scenePos)
    signal droppedOut(int index, point scenePos)

    anchors.fill: parent
    hoverEnabled: true
    preventStealing: true
    acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton

    function finish() {
        if (outside)
            area.draggedOut(-1, Qt.point(-1, -1))
        dragActive = false
        target = null
        outside = false
    }

    onPressed: mouse => {
        if (mouse.button === Qt.RightButton) {
            area.menuRequested(index)
            return
        }
        if (mouse.button !== Qt.LeftButton)
            return
        pressPos = mapToItem(list, mouse.x, mouse.y)
        finish()
        area.activated(index)
    }

    onPositionChanged: mouse => {
        if (!(mouse.buttons & Qt.LeftButton))
            return
        if (!dragActive) {
            const at = mapToItem(list, mouse.x, mouse.y)
            const dx = at.x - pressPos.x
            const dy = at.y - pressPos.y
            if (Math.abs(dx) < Qt.styleHints.startDragDistance && Math.abs(dy) < Qt.styleHints.startDragDistance)
                return
            dragActive = true
        }
        // Map the pointer before moving the tab: `mouse` is relative to where
        // the tab is now.
        const scenePos = mapToItem(null, mouse.x, mouse.y)
        const inList = mapToItem(list, mouse.x, mouse.y)
        dragPos = inList
        const wasOutside = outside
        outside = inList.x < 0 || inList.y < 0 || inList.x > list.width || inList.y > list.height
        if (outside) {
            target = null
            area.draggedOut(index, scenePos)
            return
        }
        if (wasOutside)
            area.draggedOut(-1, Qt.point(-1, -1))
        // The list may show a filtered view of the model: the delegate under
        // the pointer knows its model row.
        const p = mapToItem(list.contentItem, mouse.x, mouse.y)
        const item = list.itemAtIndex(list.indexAt(p.x, p.y))
        target = item && item.row !== index && item.visible ? item : null
    }

    onReleased: mouse => {
        const to = target ? target.row : -1
        // Dropped off the strip: say so before finish() reports the drag over.
        if (outside)
            area.droppedOut(index, mapToItem(null, mouse.x, mouse.y))
        outside = false
        finish()
        if (to >= 0)
            area.moveRequested(index, to)
    }
    onCanceled: finish()

    onClicked: mouse => {
        if (mouse.button === Qt.MiddleButton)
            area.closeRequested(index)
    }

    // Where the tab will land: a line on the target's far side.
    Rectangle {
        parent: area.list.contentItem
        visible: area.target !== null
        readonly property bool after: area.target ? area.target.row > area.index : false
        readonly property real thickness: Theme.groupLineWidth
        x: !area.target ? 0 : area.vertical ? area.target.x
           : area.target.x + (after ? area.target.width : 0) - thickness / 2
        y: !area.target ? 0 : !area.vertical ? area.target.y
           : area.target.y + (after ? area.target.height : 0) - thickness / 2
        width: area.vertical ? (area.target?.width ?? 0) : thickness
        height: area.vertical ? thickness : (area.target?.height ?? 0)
        radius: thickness / 2
        color: Theme.accent
        z: 10
    }
}
