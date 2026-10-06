pragma Singleton

import QtQuick

// Design tokens for Ion's UI. Every surface reads its colors and sizes from
// here, so the theming work can swap the palette (built-in themes, DMS,
// system accent) without touching individual components.
QtObject {
    // Palette
    readonly property color background: "#16161d"
    readonly property color surface: "#1f1f28"
    readonly property color surfaceRaised: "#2a2a37"
    readonly property color surfaceHover: "#363646"
    readonly property color text: "#dcd7ba"
    readonly property color textMuted: "#8a8980"
    readonly property color accent: "#7e9cd8"
    readonly property color border: "#2a2a37"

    // Shape and density
    readonly property int radius: 8
    readonly property int spacing: 6
    readonly property int tabHeight: 32
    readonly property int tabMaxWidth: 220
    readonly property int urlBarHeight: 32
    readonly property int iconSize: 16

    // Typography
    readonly property int fontSize: 13

    // Motion
    readonly property int animationMs: 120
}
