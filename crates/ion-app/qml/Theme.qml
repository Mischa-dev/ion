pragma Singleton

import QtQuick
import Ion

// Design tokens for Ion's UI. Every surface reads its colors and sizes from
// here. Colors come from the active theme, resolved in Rust by `ThemeEngine`
// (crates/ion-theme): a built-in theme, a DMS or manual palette file (live
// reloaded), or the system accent. Components only read tokens, so the whole
// UI re-themes together without restarting.
QtObject {
    id: theme

    // Theme selection, bound to the [theme] config section (docs/CONFIG.md).
    // source: "builtin" (default) | "dms" | "system" | "manual"
    property alias source: engine.source
    // Built-in or installed theme id for "builtin"; "auto" follows the system
    // light/dark mode.
    property alias themeName: engine.themeName
    // Palette file for "dms" / "manual"; empty uses ~/.config/ion/dms-palette.toml
    // or ~/.config/ion/palette.toml.
    property alias palettePath: engine.palettePath
    // What web pages see: "match" (default) | "system" | "darken".
    property alias pages: engine.pages

    readonly property string name: engine.name
    readonly property bool dark: engine.dark
    readonly property var themeIds: engine.themeIds
    readonly property string error: engine.error

    // Palette
    readonly property color background: engine.background
    readonly property color surface: engine.surface
    readonly property color surfaceRaised: engine.surfaceRaised
    readonly property color surfaceHover: engine.surfaceHover
    readonly property color text: engine.text
    readonly property color textMuted: engine.textMuted
    readonly property color accent: engine.accent
    readonly property color onAccent: engine.onAccent
    readonly property color border: engine.border
    readonly property color danger: engine.danger
    readonly property color warning: engine.warning
    readonly property color success: engine.success

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

    // Command palette
    readonly property int paletteWidth: 640
    readonly property int paletteMaxRows: 8
    readonly property real scrimOpacity: 0.4

    readonly property SystemPalette systemPalette: SystemPalette {}

    readonly property ThemeEngine engine: ThemeEngine {
        id: engine
        source: Config.themeSource
        themeName: Config.themeName
        palettePath: Config.revision, Config.value("theme.palette") ?? ""
        pages: Config.revision, Config.value("theme.pages") ?? ""
        // An unknown scheme (common on Linux without a platform theme) counts
        // as dark, Ion's default look.
        systemDark: Qt.styleHints.colorScheme !== Qt.ColorScheme.Light
        systemAccent: theme.systemPalette.accent
        onErrorChanged: if (error.length > 0) console.warn("Ion theme:", error)
    }
}
