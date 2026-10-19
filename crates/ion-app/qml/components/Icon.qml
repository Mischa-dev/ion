import QtQuick
import QtQuick.Shapes
import Ion

// One of Ion's line icons (IconSet.qml), drawn as vector strokes so it stays
// crisp at any size and scale and takes any theme color.
Item {
    id: icon

    // A name from IconSet.icons, such as "back" or "star-filled".
    property string name
    property color color: Theme.text
    property real size: Theme.iconSize

    readonly property var spec: IconSet.icons[name] ?? null

    implicitWidth: size
    implicitHeight: size

    Shape {
        visible: icon.spec !== null
        width: 24
        height: 24
        anchors.centerIn: parent
        scale: Math.min(icon.width, icon.height) / 24
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: icon.spec && !icon.spec.fill ? icon.color : "transparent"
            strokeWidth: icon.spec?.weight ?? Theme.iconStroke
            fillColor: icon.spec && icon.spec.fill ? icon.color : "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin

            PathSvg { path: icon.spec?.d ?? "" }
        }
    }

    Behavior on color { ColorAnimation { duration: Theme.animationMs } }
}
