import QtQuick
import QtQuick.Controls
import Ion

// A menu in Ion's look: a floating rounded surface with a soft shadow, roomy
// rows and a quick grow-in. Pair with IonMenuItem and IonMenuSeparator;
// submenus get IonMenuItem rows too.
Menu {
    id: menu

    // Leave an icon's width before every label, so rows with and without an
    // icon line up. Set it on menus where some rows have one.
    property bool icons: false

    // Drawn by Qt Quick in every style. On macOS the default is a native menu,
    // which ignores the delegate, background and transitions below.
    popupType: Popup.Item
    padding: Theme.spacing
    implicitWidth: Theme.tabMaxWidth + Theme.spacing * 4
    delegate: IonMenuItem {}
    transformOrigin: Item.Top

    background: Item {
        implicitWidth: Theme.tabMaxWidth + Theme.spacing * 4

        IonShadow {
            anchors.fill: parent
            radius: Theme.radiusLarge
        }
        Rectangle {
            anchors.fill: parent
            radius: Theme.radiusLarge
            color: Theme.surface
            border.color: Theme.border
            border.width: Theme.hairline
        }
    }

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs; easing.type: Theme.easeOut }
        NumberAnimation { property: "scale"; from: Theme.popupStartScale; to: 1; duration: Theme.animationMs; easing.type: Theme.easeOut }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationFastMs }
    }
}
