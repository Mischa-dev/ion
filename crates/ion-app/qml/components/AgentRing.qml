import QtQuick
import Ion

// An agent's mark: a small ring in its color that turns while the agent is
// working and sits still otherwise. The only motion agents have
// (docs: agent-feel design). It follows the animation speed; with
// animations off it shows work as a still, dashed ring instead.
Item {
    id: ring

    property color color: Theme.accent
    property bool working: false
    property int size: Theme.iconSize - 4
    // Animations are off: show work without motion.
    readonly property bool still: Theme.motionScale === 0

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
            // A gap while working shows the turn; whole when at rest. With
            // animations off, working is a dashed ring that doesn't move.
            if (ring.working && ring.still)
                ctx.setLineDash([2, 2])
            const sweep = ring.working && !ring.still ? Math.PI * 1.5 : Math.PI * 2
            ctx.arc(width / 2, height / 2, r, 0, sweep)
            ctx.stroke()
        }
        Connections {
            target: ring
            function onColorChanged() { canvas.requestPaint() }
            function onWorkingChanged() { canvas.requestPaint() }
            function onStillChanged() { canvas.requestPaint() }
        }
    }

    RotationAnimator on rotation {
        running: ring.working && ring.visible && !ring.still
        from: 0
        to: 360
        // About one turn every 2.4 s at normal speed.
        duration: Math.max(1, Math.round(2400 * Theme.motionScale))
        loops: Animation.Infinite
        onRunningChanged: if (!running) ring.rotation = 0
    }
}
