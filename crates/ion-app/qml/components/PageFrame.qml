import QtQuick
import QtQuick.Shapes
import Ion

// Turns the page area into a card: rounded corners cut in the chrome's color
// and a hairline edge, drawn over the web view (rounding the view itself
// would cost an offscreen layer per frame). Put it on top of the page area,
// with an IonShadow under it.
Item {
    id: frame

    property real radius: Theme.radiusLarge
    // The color around the card, which the corners are filled with.
    property color outside: Theme.background

    // One corner: the square minus a quarter circle, rotated into place.
    component Corner: Shape {
        width: frame.radius
        height: frame.radius
        visible: frame.radius > 0
        preferredRendererType: Shape.CurveRenderer
        transformOrigin: Item.Center

        ShapePath {
            strokeColor: "transparent"
            fillColor: frame.outside
            startX: 0
            startY: 0
            PathLine { x: frame.radius; y: 0 }
            PathArc {
                x: 0
                y: frame.radius
                radiusX: frame.radius
                radiusY: frame.radius
                direction: PathArc.Counterclockwise
            }
            PathLine { x: 0; y: 0 }
        }
    }

    Corner { anchors.left: parent.left; anchors.top: parent.top }
    Corner { anchors.right: parent.right; anchors.top: parent.top; rotation: 90 }
    Corner { anchors.right: parent.right; anchors.bottom: parent.bottom; rotation: 180 }
    Corner { anchors.left: parent.left; anchors.bottom: parent.bottom; rotation: 270 }

    Rectangle {
        anchors.fill: parent
        radius: frame.radius
        color: "transparent"
        border.width: Theme.hairline
        border.color: Theme.border
    }
}
