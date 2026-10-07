import QtQuick
import QtQuick.Controls
import Ion

// A menu in Ion's look: themed surface, rounded rows, roomy hit targets.
// Pair with IonMenuItem and IonMenuSeparator; submenus get IonMenuItem rows too.
Menu {
    id: menu

    padding: Theme.spacing / 2
    implicitWidth: Theme.tabMaxWidth
    delegate: IonMenuItem {}

    background: Rectangle {
        implicitWidth: Theme.tabMaxWidth
        radius: Theme.radius
        color: Theme.surface
        border.color: Theme.border
        border.width: Theme.hairline
    }

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
    }
}
