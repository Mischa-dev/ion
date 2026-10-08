import QtQuick
import Ion

// An agent's mark: a small ring in its color that turns while the agent is
// working and sits still otherwise. The only motion agents have
// (docs: agent-feel design), so it keeps turning with animations off, like
// a loading spinner: it reports status.
Item {
    id: ring

    property color color: Theme.accent
    property bool working: false
    property int size: Theme.iconSize - 4

    implicitWidth: size
    implicitHeight: size

    Canvas {
        id: canvas
        anchors.fill: parent
        onPaint: {
            const ctx = getContext("2d")
            ctx.reset()
            const line = 1.5
            const r = (Math.min(width, height) - line) / 2
            ctx.lineWidth = line
            ctx.strokeStyle = ring.color
            ctx.beginPath()
            // A gap while working shows the turn; whole when at rest.
            const sweep = ring.working ? Math.PI * 1.5 : Math.PI * 2
            ctx.arc(width / 2, height / 2, r, 0, sweep)
            ctx.stroke()
        }
        Connections {
            target: ring
            function onColorChanged() { canvas.requestPaint() }
            function onWorkingChanged() { canvas.requestPaint() }
        }
    }

    RotationAnimator on rotation {
        running: ring.working && ring.visible
        from: 0
        to: 360
        duration: 2400
        loops: Animation.Infinite
        onRunningChanged: if (!running) ring.rotation = 0
    }
}
