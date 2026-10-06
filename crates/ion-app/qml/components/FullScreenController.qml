import QtQuick
import QtQuick.Window
import QtWebEngine

// Fullscreen for pages (video players, presentations) and for the window
// (F11, or Ctrl+Cmd+F on macOS). Main.qml hides the toolbars while
// `active` is true. Esc leaves page fullscreen.
Item {
    id: controller

    required property Window window
    // The current tab's BrowserTab; may be null.
    property WebEngineView view

    // A page asked for fullscreen (requestFullscreen()).
    readonly property bool pageActive: pageView !== null
    // The person asked for fullscreen with the shortcut.
    property bool windowActive: false
    readonly property bool active: pageActive || windowActive

    property WebEngineView pageView: null
    property int restoreVisibility: Window.Windowed

    function enter() {
        if (window.visibility !== Window.FullScreen)
            restoreVisibility = window.visibility
        window.showFullScreen()
    }

    function leave() {
        if (!active && window.visibility === Window.FullScreen)
            window.visibility = restoreVisibility
    }

    function toggleWindow() {
        if (pageActive) {
            pageView.triggerWebAction(WebEngineView.ExitFullScreen)
            return
        }
        windowActive = !windowActive
        if (windowActive)
            enter()
        else
            leave()
    }

    Connections {
        target: controller.view
        function onFullScreenRequested(request) {
            request.accept()
            if (request.toggleOn) {
                controller.pageView = controller.view
                controller.enter()
            } else {
                controller.pageView = null
                controller.leave()
            }
        }
    }

    // Switching tabs takes the page out of fullscreen.
    onViewChanged: {
        if (pageView && pageView !== view) {
            pageView.triggerWebAction(WebEngineView.ExitFullScreen)
            pageView = null
            leave()
        }
    }

    // Leaving fullscreen through the window manager resets our state.
    Connections {
        target: controller.window
        function onVisibilityChanged(visibility) {
            if (visibility === Window.FullScreen || !controller.active)
                return
            controller.windowActive = false
            if (controller.pageView) {
                controller.pageView.triggerWebAction(WebEngineView.ExitFullScreen)
                controller.pageView = null
            }
        }
    }

    Shortcut {
        sequences: [StandardKey.FullScreen, "F11"]
        onActivated: controller.toggleWindow()
    }
    Shortcut {
        sequences: ["Esc"]
        enabled: controller.pageActive
        onActivated: controller.pageView.triggerWebAction(WebEngineView.ExitFullScreen)
    }
}
