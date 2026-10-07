import QtQuick

// Mouse handling for one tab in TabStrip or VerticalTabStrip: left press
// activates the tab, middle click closes it, right click opens its menu, and
// dragging it along the strip reorders it live. The strip's model moves the
// row as the pointer passes over a neighbour, so the dragged tab always stays
// under the pointer.
MouseArea {
    id: area

    required property ListView list   // the strip's tab list
    required property int index       // the tab's row

    readonly property bool dragging: dragActive
    property bool dragActive: false
    property point pressPos

    signal activated(int index)
    signal closeRequested(int index)
    signal moveRequested(int from, int to)
    signal menuRequested(int index)

    anchors.fill: parent
    hoverEnabled: true
    preventStealing: true
    acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton

    onPressed: mouse => {
        if (mouse.button === Qt.RightButton) {
            area.menuRequested(index)
            return
        }
        if (mouse.button !== Qt.LeftButton)
            return
        pressPos = Qt.point(mouse.x, mouse.y)
        dragActive = false
        area.activated(index)
    }

    onPositionChanged: mouse => {
        if (!(mouse.buttons & Qt.LeftButton))
            return
        if (!dragActive) {
            const dx = mouse.x - pressPos.x
            const dy = mouse.y - pressPos.y
            if (Math.abs(dx) < Qt.styleHints.startDragDistance && Math.abs(dy) < Qt.styleHints.startDragDistance)
                return
            dragActive = true
        }
        const p = mapToItem(list.contentItem, mouse.x, mouse.y)
        const target = list.indexAt(p.x, p.y)
        if (target >= 0 && target !== index)
            area.moveRequested(index, target)
    }

    onReleased: dragActive = false
    onCanceled: dragActive = false

    onClicked: mouse => {
        if (mouse.button === Qt.MiddleButton)
            area.closeRequested(index)
    }
}
