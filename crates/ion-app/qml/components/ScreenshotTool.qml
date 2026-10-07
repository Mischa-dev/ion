import QtQuick
import QtQuick.Layouts
import QtWebEngine
import Ion

// Page screenshots: the visible area, or the whole page stitched from tiles
// grabbed while scrolling. Each one is saved as a PNG in the downloads folder
// (honouring `downloads.folders`) and copied to the clipboard, then a short
// toast says where it went. Lives inside a BrowserTab.
Item {
    id: tool

    required property WebEngineView view

    property bool busy: false
    // Full-page capture state.
    property var offsets: []
    property int tileIndex: 0
    property var canvas: null
    property real pageHeight: 0
    property real viewportHeight: 0
    property real startScroll: 0
    // Where the last screenshot was saved, for the toast's Open button.
    property url savedFile
    property string savedDir
    property bool toastShown: false

    function capture(fullPage) {
        if (busy || !view.visible)
            return
        busy = true
        if (!fullPage) {
            grab(image => finish(image))
            return
        }
        view.runJavaScript(
            // The scrollbar would show on every tile, so it goes first.
            `const style = document.createElement("style")
            style.id = "ion-screenshot-style"
            style.textContent = "html { scrollbar-width: none !important }"
            document.documentElement.append(style);
            ({ h: document.documentElement.scrollHeight, v: innerHeight, y: scrollY })`,
            size => {
                if (!size || !(size.v > 0)) {
                    // Pages that block scripts still get the visible area.
                    grab(image => finish(image))
                    return
                }
                pageHeight = Screenshot.capturedHeight(size.h, size.v)
                viewportHeight = size.v
                startScroll = size.y
                offsets = Screenshot.tileOffsets(size.h, size.v)
                tileIndex = 0
                canvas = null
                nextTile()
            })
    }

    function grab(done) {
        if (!view.grabToImage(result => done(result.image))) {
            busy = false
            showToast(qsTr("Couldn't take a screenshot"), false)
        }
    }

    // Scroll to the next offset, give the page a moment to paint, grab it.
    function nextTile() {
        const index = tileIndex
        if (index >= offsets.length) {
            const image = canvas
            restorePage()
            finish(image)
            return
        }
        // Fixed and sticky elements (headers, cookie bars) would repeat on
        // every tile, so they are hidden after the first one.
        const hideFixed = index === 1 ? `
            for (const el of document.querySelectorAll("body *")) {
                const p = getComputedStyle(el).position
                if (p === "fixed" || p === "sticky") {
                    el.dataset.ionShotVisibility = el.style.visibility
                    el.style.setProperty("visibility", "hidden", "important")
                }
            }` : ""
        view.runJavaScript(`${hideFixed}
            window.scrollTo({ top: ${offsets[index]}, behavior: "instant" })`,
            () => settle.restart())
    }

    function restorePage() {
        view.runJavaScript(`
            document.getElementById("ion-screenshot-style")?.remove()
            for (const el of document.querySelectorAll("[data-ion-shot-visibility]")) {
                el.style.visibility = el.dataset.ionShotVisibility
                delete el.dataset.ionShotVisibility
            }
            window.scrollTo({ top: ${startScroll}, behavior: "instant" })`)
        offsets = []
        canvas = null
    }

    function finish(image) {
        busy = false
        const name = Screenshot.fileName(view.url, Qt.formatDateTime(new Date(), "yyyy-MM-dd hh.mm.ss"))
        const dir = Downloads.targetDirectory(name, "image/png", view.profile.downloadPath)
        Screenshot.copy(image)
        if (Screenshot.save(image, dir + "/" + name)) {
            savedFile = Downloads.fileUrl(dir, name)
            savedDir = dir
            showToast(qsTr("Screenshot saved and copied"), true)
        } else {
            showToast(qsTr("Screenshot copied; saving to %1 failed").arg(dir), false)
        }
    }

    function showToast(text, canOpen) {
        toastText.text = text
        openButton.visible = canOpen
        toastShown = true
        hideTimer.restart()
    }

    Timer {
        id: settle
        interval: 250
        // The grabbed image only lives during the callback, so it is
        // painted onto the page image right away.
        onTriggered: tool.grab(image => {
            tool.canvas = Screenshot.paste(tool.canvas, image, tool.offsets[tool.tileIndex],
                                           tool.viewportHeight, tool.pageHeight)
            tool.tileIndex++
            tool.nextTile()
        })
    }

    Timer {
        id: hideTimer
        interval: 4000
        onTriggered: tool.toastShown = false
    }

    // Stop a full-page capture if the page navigates away mid-way.
    Connections {
        target: tool.view
        function onLoadingChanged(info) {
            if (tool.busy && info.status === WebEngineView.LoadStartedStatus) {
                settle.stop()
                tool.busy = false
                tool.offsets = []
                tool.canvas = null
            }
        }
    }

    anchors.fill: parent
    z: 15

    Rectangle {
        id: toast
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spacing * 3
        width: row.implicitWidth + Theme.spacing * 3
        height: row.implicitHeight + Theme.spacing * 2
        radius: Theme.radius
        color: Theme.surface
        border.color: Theme.border
        border.width: 1
        // Hidden at once while capturing so it never ends up in a screenshot.
        opacity: tool.toastShown ? 1 : 0
        visible: opacity > 0 && !tool.busy

        Behavior on opacity { NumberAnimation { duration: Theme.animationMs } }

        HoverHandler { onHoveredChanged: if (hovered) hideTimer.stop(); else hideTimer.restart() }

        RowLayout {
            id: row
            anchors.centerIn: parent
            spacing: Theme.spacing * 1.5

            Text {
                id: toastText
                color: Theme.text
                font.pixelSize: Theme.fontSize
            }
            DialogButton {
                id: openButton
                text: qsTr("Open")
                onClicked: {
                    Qt.openUrlExternally(tool.savedFile)
                    tool.toastShown = false
                }
            }
            DialogButton {
                visible: openButton.visible
                text: qsTr("Show folder")
                onClicked: {
                    Qt.openUrlExternally(Downloads.fileUrl(tool.savedDir, ""))
                    tool.toastShown = false
                }
            }
        }
    }
}
