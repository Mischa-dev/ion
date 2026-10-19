import QtQuick
import Ion

// A soft drop shadow for a floating surface (menus, popups, dialogs, the page
// card). Put it behind the surface and anchor it to the same rectangle. Built
// from stacked translucent rounded rectangles rather than a shader, so it
// draws the same with every Qt Quick backend, including software rendering.
Item {
    id: shadow

    // Corner radius of the surface it sits under.
    property real radius: Theme.radiusLarge
    // How far the shadow reaches past the edges, and how far it drops.
    property real spread: Theme.shadowRadius
    property real offset: Theme.shadowOffset
    // Overall strength, 0…1.
    property real strength: 1

    readonly property int layers: 8

    Repeater {
        model: shadow.layers

        Rectangle {
            required property int index
            // Outermost layer first; each one smaller and adding to the
            // darkness towards the middle.
            readonly property real reach: shadow.spread * (1 - index / shadow.layers)
            x: -reach
            y: -reach + shadow.offset * (1 - index / shadow.layers * 0.5)
            width: shadow.width + reach * 2
            height: shadow.height + reach * 2
            radius: shadow.radius + reach
            color: Theme.shadow
            opacity: shadow.strength * 0.9 / shadow.layers * (index + 1) / shadow.layers * 2
        }
    }
}
