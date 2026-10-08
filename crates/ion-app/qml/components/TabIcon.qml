import QtQuick
import Ion

// A tab's favicon, or a small spinning arc while its page loads. Collapses to
// nothing for tabs with neither (new tabs, sites without an icon). While an
// agent changes the tab's page, the agent's ring turns around the icon.
Item {
    id: root

    property var view   // BrowserTab, may be null (tabs restored but not shown yet)
    property string savedIcon   // the icon saved with the session, for unloaded tabs

    readonly property bool loading: view?.loading ?? false
    readonly property bool hasIcon: favicon.status === Image.Ready && favicon.implicitWidth > 0
    readonly property bool agentActing: {
        Agent.revision
        return view ? Agent.actingOn(view.tabId) : false
    }

    implicitWidth: loading || hasIcon || agentActing ? Theme.iconSize : 0
    implicitHeight: Theme.iconSize

    Image {
        id: favicon
        anchors.fill: parent
        visible: !root.loading
        // Blanked for a moment to ask again (see retryTimer).
        property bool retrying: false
        source: {
            if (retrying)
                return ""
            const live = root.view?.icon?.toString() ?? ""
            return live.length > 0 ? live : root.savedIcon
        }
        cache: false
        sourceSize.width: Theme.iconSize
        sourceSize.height: Theme.iconSize
        fillMode: Image.PreserveAspectFit
        smooth: true
        asynchronous: true
    }

    // The favicon provider answers with an empty image until the profile has
    // the icon, which for tabs restored at startup can be a moment later. Ask
    // again a few times rather than keep the blank.
    Timer {
        id: retryTimer
        property int attempts: 0
        interval: 500
        repeat: true
        running: favicon.status === Image.Ready && favicon.implicitWidth === 0
                 && favicon.source.toString().length > 0 && attempts < 10
        onTriggered: {
            attempts++
            favicon.retrying = true
            favicon.retrying = false
        }
    }

    Canvas {
        id: spinner
        anchors.fill: parent
        visible: root.loading
        onPaint: {
            const ctx = getContext("2d")
            const r = width / 2 - 1.5
            ctx.reset()
            ctx.lineWidth = 2
            ctx.lineCap = "round"
            ctx.strokeStyle = Theme.accent
            ctx.beginPath()
            ctx.arc(width / 2, height / 2, r, 0, Math.PI * 1.4)
            ctx.stroke()
        }

        Connections {
            target: Theme
            function onAccentChanged() { spinner.requestPaint() }
        }

        RotationAnimator on rotation {
            running: spinner.visible
            from: 0
            to: 360
            loops: Animation.Infinite
            duration: Theme.spinnerMs
        }
    }

    AgentRing {
        anchors.centerIn: parent
        visible: root.agentActing
        working: true
        size: Theme.iconSize + 6
    }
}
