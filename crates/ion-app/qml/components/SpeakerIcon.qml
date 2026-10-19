import QtQuick
import QtQuick.Shapes
import Ion

// A small speaker: with sound waves while playing, struck through when muted.
// Drawn on a 16-unit grid and scaled to `size`.
Item {
    id: root

    property bool muted: false
    property color color: Theme.text
    property int size: Theme.iconSize

    implicitWidth: size
    implicitHeight: size

    Shape {
        anchors.fill: parent
        scale: root.size / 16
        transformOrigin: Item.TopLeft
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeWidth: 0
            fillColor: root.color
            startX: 2; startY: 6
            PathLine { x: 5; y: 6 }
            PathLine { x: 9; y: 2.5 }
            PathLine { x: 9; y: 13.5 }
            PathLine { x: 5; y: 10 }
            PathLine { x: 2; y: 10 }
            PathLine { x: 2; y: 6 }
        }
        ShapePath {
            strokeWidth: root.muted ? 0 : 1.4
            strokeColor: root.muted ? "transparent" : root.color
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            startX: 11; startY: 5.8
            PathArc { x: 11; y: 10.2; radiusX: 2.6; radiusY: 2.6 }
            PathMove { x: 12.6; y: 3.6 }
            PathArc { x: 12.6; y: 12.4; radiusX: 5; radiusY: 5 }
        }
        ShapePath {
            strokeWidth: root.muted ? 1.4 : 0
            strokeColor: root.muted ? root.color : "transparent"
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            startX: 11; startY: 6
            PathLine { x: 15; y: 10 }
            PathMove { x: 15; y: 6 }
            PathLine { x: 11; y: 10 }
        }
    }
}
