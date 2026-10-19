import QtQuick
import Ion

// The floating surface behind Ion's popups, panels and dialogs: a rounded
// card with a hairline edge and a soft shadow. Use it as a Popup's background.
Item {
    id: surface

    property real radius: Theme.radiusLarge
    property color color: Theme.surface
    // How strongly it lifts off what is under it, 0…1.
    property real elevation: 1

    IonShadow {
        anchors.fill: parent
        radius: surface.radius
        strength: surface.elevation
    }
    Rectangle {
        anchors.fill: parent
        radius: surface.radius
        color: surface.color
        border.color: Theme.border
        border.width: Theme.hairline
    }
}
