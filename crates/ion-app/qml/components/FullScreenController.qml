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
        // Remember the state from before the first of page and window
        // fullscreen, so a window that was already fullscreen (the macOS green
        // button, the window manager) stays fullscreen when both end.
        if (!(pageActive && windowActive))
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
        // Fullscreen entered outside Ion (the macOS green button, the window
        // manager): the shortcut takes the window out of it.
        if (!windowActive && window.visibility === Window.FullScreen) {
            window.showNormal()
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

    // A fullscreen tab that is closed clears pageView on its own; restore the
    // window then too.
    onPageViewChanged: {
        if (!pageView)
            leave()
    }

    // Switching tabs takes the page out of fullscreen. The old tab's exit
    // request would arrive after the Connections above moved on to the new
    // view, so reset its fullscreen state directly instead of asking.
    onViewChanged: {
        if (pageView && pageView !== view) {
            pageView.fullScreenCancelled()
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
