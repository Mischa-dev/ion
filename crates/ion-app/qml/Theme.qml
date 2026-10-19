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
    // Derived colors. Translucent tints of the palette, so they sit well on
    // any surface: hover and pressed washes, the accent's soft fill, the
    // keyboard focus ring and drop shadows.
    function alpha(c, a) { return Qt.rgba(c.r, c.g, c.b, a) }
    readonly property color hoverOverlay: alpha(text, dark ? 0.07 : 0.06)
    readonly property color pressOverlay: alpha(text, dark ? 0.12 : 0.1)
    readonly property color accentSoft: alpha(accent, dark ? 0.2 : 0.14)
    readonly property color focusRing: alpha(accent, 0.6)
    readonly property color shadow: Qt.rgba(0, 0, 0, dark ? 0.5 : 0.16)
    // A light top-down sheen on accent-filled buttons.
    readonly property color sheen: Qt.rgba(1, 1, 1, 0.1)
    readonly property color sheenHover: Qt.rgba(1, 1, 1, 0.16)
    readonly property color sheenEnd: Qt.rgba(1, 1, 1, 0)
    // Quieter than textMuted: hints, shortcuts, placeholder detail.
    readonly property color textSubtle: alpha(textMuted, 0.75)

    // What a web page without its own background is drawn on. Not themed:
    // pages assume the web's default white.
    readonly property color pageCanvas: "white"
    // Picture-in-picture draws over video, so its colors don't follow the
    // theme: light controls on a dark wash.
    readonly property color videoCanvas: "black"
    readonly property color onVideo: "white"
    readonly property color videoScrim: Qt.rgba(0, 0, 0, 0.6)

    // Shape and density, from the [ui] config section (docs/CONFIG.md).
    readonly property bool compact: Config.density === "compact"
    readonly property int radius: Math.max(0, Config.cornerRadius)
    // Small controls inside a rounded surface (menu rows, chips), and large
    // surfaces (popups, dialogs, the page card). All follow `ui.cornerRadius`.
    readonly property int radiusSmall: Math.round(radius * 0.75)
    readonly property int radiusLarge: Math.round(radius * 1.5)
    readonly property int spacing: compact ? 4 : 6
    readonly property int tabHeight: compact ? 28 : 32
    readonly property int tabMinWidth: compact ? 64 : 80
    readonly property int tabMaxWidth: 220
    // A collapsed vertical tab sidebar: just wide enough for the favicons.
    readonly property int sidebarCollapsedWidth: iconSize + spacing * 6
    readonly property int urlBarHeight: compact ? 28 : 32
    readonly property int iconSize: 16
    // Line weight of Ion's icons (Icon.qml), in a 24-unit grid.
    readonly property real iconStroke: 2
    readonly property int hairline: 1

    // Typography. The typeface is the application font (Inter on Linux, the
    // system font on macOS; cpp/fonts.cpp), so only sizes and weights live here.
    readonly property int fontSize: 13
    // Captions, shortcuts, badges.
    readonly property int fontSizeSmall: 11
    // Section headings in popups and dialog titles.
    readonly property int fontSizeLarge: 15
    // The new-tab clock and other big display text.
    readonly property int displayFontSize: 64
    readonly property int weightMedium: Font.Medium
    readonly property int weightBold: Font.DemiBold
    // Headings on Ion's own pages, such as the error page.
    readonly property int titleFontSize: 22
    // Widest a column of running text gets on Ion's own pages.
    readonly property int readableWidth: 520
    // The smallest text shrinks to when fitting a tight space, such as a
    // workspace icon in the collapsed sidebar.
    readonly property int fontSizeMin: 8

    // Motion. `ui.animations` turns it off or changes its speed; 0 disables
    // every Behavior and Transition that reads these.
    readonly property real motionScale: Config.animationsEnabled && Config.animationSpeed > 0
        ? 1 / Config.animationSpeed : 0
    readonly property int animationMs: Math.round(120 * motionScale)
    readonly property int animationSlowMs: Math.round(220 * motionScale)
    // Quick feedback such as a button press.
    readonly property int animationFastMs: Math.round(80 * motionScale)
    // Easing for things arriving (popups opening, items appearing) and for
    // things moving between two places.
    readonly property int easeOut: Easing.OutCubic
    readonly property int easeInOut: Easing.InOutCubic
    // How far a popup grows into place as it opens.
    readonly property real popupStartScale: 0.96

    // Elevation: how far a popup's soft shadow reaches (IonShadow.qml).
    readonly property int shadowRadius: 18
    readonly property int shadowOffset: 6
    // Loading spinners keep turning with animations off: they report status.
    readonly property int spinnerMs: 800
    // How long a transient notice stays on screen.
    readonly property int noticeMs: 2500
    // How long the pointer rests on a collapsed sidebar before it expands, so
    // passing over it on the way somewhere else doesn't.
    readonly property int hoverDelayMs: 250
    // How long a permission prompt's Allow stays disabled after it appears,
    // so a click already on its way can't land on it (docs/SAFETY.md). Not
    // scaled by motion settings: it is a safety delay, not an animation.
    readonly property int promptArmMs: 400
    // How long a web notification toast stays up, long enough to read a
    // sentence. Fixed: reading time shouldn't follow animation speed.
    readonly property int toastMs: 6000

    // The agent sidebar beside the page.
    readonly property int agentSidebarWidth: 380
    readonly property int agentSidebarMinWidth: 280

    // Command palette
    readonly property int paletteWidth: 640
    readonly property int paletteMaxRows: 8
    readonly property real scrimOpacity: 0.4
    // Colors offered for workspaces. A workspace without one uses `accent`.
    readonly property var workspaceColors: [
        { name: qsTr("Red"), value: "#e5484d" },
        { name: qsTr("Orange"), value: "#f76b15" },
        { name: qsTr("Yellow"), value: "#ffc53d" },
        { name: qsTr("Green"), value: "#46a758" },
        { name: qsTr("Teal"), value: "#12a594" },
        { name: qsTr("Blue"), value: "#0090ff" },
        { name: qsTr("Purple"), value: "#8e4ec6" },
        { name: qsTr("Pink"), value: "#d6409f" }
    ]
    readonly property int workspaceDotSize: 10

    // Things that are present but asleep, like a suspended tab's icon.
    readonly property real dimmedOpacity: 0.45
    // A color laid thinly over a surface, like a tab group's header.
    readonly property real tintOpacity: 0.16
    // The colored line that ties a tab group's tabs together.
    readonly property int groupLineWidth: 2
    // Tab hover previews: the page picture's width, and how often the tab on
    // screen refreshes its picture.
    readonly property int previewWidth: 260
    readonly property int snapshotMs: 10000

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
