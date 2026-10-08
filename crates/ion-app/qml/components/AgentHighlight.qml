import QtQuick
import Ion

// Where an agent just clicked or typed: a soft outline over the element in
// the agent's color that fades away, so the person sees what changed
// without anything moving. Drawn over the web view rather than in the page,
// which never sees it. Destroys itself.
Rectangle {
    id: highlight

    property color tint: Theme.accent
    // How long it stays before fading, in ms; fixed, since it is how long
    // the person has to notice it.
    property int holdMs: 700

    radius: Theme.radius
    color: Qt.rgba(tint.r, tint.g, tint.b, 0.14)
    border.width: 2
    border.color: tint

    SequentialAnimation on opacity {
        running: true
        PauseAnimation { duration: highlight.holdMs }
        NumberAnimation { to: 0; duration: Math.max(1, Theme.animationSlowMs * 2) }
        ScriptAction { script: highlight.destroy() }
    }
}
