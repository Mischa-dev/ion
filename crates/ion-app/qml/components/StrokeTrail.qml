import QtQuick
import Ion

// The line a mouse gesture draws over the page, and the name of the command
// letting go would run (`Gestures`).
Item {
    id: root

    property bool active: false
    // What letting go runs; empty while the stroke matches nothing.
    property string label

    property var points: []
    property point lastPoint

    // A point in window coordinates.
    function add(x, y) {
        const p = mapFromItem(null, x, y)
        points.push(p)
        lastPoint = p
        canvas.requestPaint()
    }

    onActiveChanged: if (!active) { points = []; canvas.requestPaint() }

    anchors.fill: parent
    visible: active

    Canvas {
        id: canvas
        anchors.fill: parent
        onPaint: {
            const ctx = getContext("2d")
            ctx.reset()
            if (root.points.length < 2)
                return
            ctx.strokeStyle = Theme.accent
            ctx.lineWidth = Theme.spacing / 2
            ctx.lineCap = "round"
            ctx.lineJoin = "round"
            ctx.globalAlpha = 0.8
            ctx.beginPath()
            ctx.moveTo(root.points[0].x, root.points[0].y)
            for (const p of root.points.slice(1))
                ctx.lineTo(p.x, p.y)
            ctx.stroke()
        }
    }

    Rectangle {
        readonly property point last: root.lastPoint
        visible: root.label.length > 0
        x: Math.min(root.width - width - Theme.spacing, last.x + Theme.spacing * 2)
        y: Math.max(Theme.spacing, last.y - height - Theme.spacing * 2)
        width: text.implicitWidth + Theme.spacing * 4
        height: Theme.tabHeight
        radius: Theme.radius
        color: Theme.surface
        border.width: Theme.hairline
        border.color: Theme.border

        Text {
            id: text
            anchors.centerIn: parent
            text: root.label
            color: Theme.text
            font.pixelSize: Theme.fontSize
        }
    }
}
