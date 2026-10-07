import QtQuick
import Ion

// A tab's favicon, or a small spinning arc while its page loads. Collapses to
// nothing for tabs with neither (new tabs, sites without an icon).
Item {
    id: root

    property var view   // BrowserTab, may be null (tabs restored but not shown yet)

    readonly property bool loading: view?.loading ?? false
    readonly property bool hasIcon: favicon.status === Image.Ready

    implicitWidth: loading || hasIcon ? Theme.iconSize : 0
    implicitHeight: Theme.iconSize

    Image {
        id: favicon
        anchors.fill: parent
        visible: !root.loading
        source: root.view?.icon ?? ""
        sourceSize.width: Theme.iconSize
        sourceSize.height: Theme.iconSize
        fillMode: Image.PreserveAspectFit
        smooth: true
        asynchronous: true
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
}
