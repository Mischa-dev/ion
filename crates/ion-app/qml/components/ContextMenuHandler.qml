import QtQuick
import QtQuick.Controls
import QtWebEngine
import Ion

// Ion's themed page context menu. Rust (`Basics.contextMenuItems`) decides
// which entries fit what was clicked; this file gives each entry its label
// and action. Lives inside a BrowserTab.
Menu {
    id: menu

    required property WebEngineView view
    // Turns selected text into a search or address URL.
    required property var omnibox

    // Asks the window to open `target` in a new tab.
    signal openInNewTab(url target)

    // Copied from the request, which the engine may reuse once handled.
    property string selectedText
    property url mediaUrl

    function show(request) {
        request.accepted = true
        selectedText = request.selectedText
        mediaUrl = request.mediaUrl
        while (menu.count > 0)
            menu.takeItem(0).destroy()

        const hasSelection = request.selectedText.length > 0
        const ids = Basics.contextMenuItems(
            request.linkUrl.toString().length > 0,
            request.mediaType,
            Number(request.mediaFlags),
            request.isContentEditable,
            Number(request.editFlags),
            hasSelection)
        for (const id of ids) {
            if (id === "-")
                menu.addItem(separatorComponent.createObject(menu))
            else
                menu.addItem(itemComponent.createObject(menu, { actionId: id, text: label(id), enabled: available(id) }))
        }
        menu.popup(request.position.x, request.position.y)
    }

    function label(id) {
        switch (id) {
        case "back": return qsTr("Back")
        case "forward": return qsTr("Forward")
        case "reload": return qsTr("Reload")
        case "openLinkInNewTab": return qsTr("Open Link in New Tab")
        case "copyLink": return qsTr("Copy Link")
        case "saveLink": return qsTr("Save Link As…")
        case "openImageInNewTab": return qsTr("Open Image in New Tab")
        case "copyImage": return qsTr("Copy Image")
        case "copyImageAddress": return qsTr("Copy Image Address")
        case "saveImage": return qsTr("Save Image As…")
        case "playPause": return qsTr("Play / Pause")
        case "mute": return qsTr("Mute / Unmute")
        case "loop": return qsTr("Loop")
        case "showControls": return qsTr("Show Controls")
        case "copyMediaAddress": return qsTr("Copy Media Address")
        case "saveMedia": return qsTr("Save Media As…")
        case "undo": return qsTr("Undo")
        case "redo": return qsTr("Redo")
        case "cut": return qsTr("Cut")
        case "copy": return qsTr("Copy")
        case "paste": return qsTr("Paste")
        case "pasteAsPlainText": return qsTr("Paste as Plain Text")
        case "selectAll": return qsTr("Select All")
        case "searchSelection": {
            const preview = Basics.selectionPreview(selectedText)
            return omnibox.isSearch(selectedText)
                ? qsTr("Search for “%1”").arg(preview)
                : qsTr("Go to %1").arg(preview)
        }
        case "savePage": return qsTr("Save Page As…")
        case "screenshot": return qsTr("Take Screenshot")
        case "viewSource": return qsTr("View Page Source")
        }
        return id
    }

    function available(id) {
        switch (id) {
        case "back": return view.canGoBack
        case "forward": return view.canGoForward
        }
        return true
    }

    function run(id) {
        const v = view
        switch (id) {
        case "back": v.goBack(); break
        case "forward": v.goForward(); break
        case "reload": v.reload(); break
        case "openLinkInNewTab": v.triggerWebAction(WebEngineView.OpenLinkInNewTab); break
        case "copyLink": v.triggerWebAction(WebEngineView.CopyLinkToClipboard); break
        case "saveLink": v.triggerWebAction(WebEngineView.DownloadLinkToDisk); break
        case "openImageInNewTab": menu.openInNewTab(mediaUrl); break
        case "copyImage": v.triggerWebAction(WebEngineView.CopyImageToClipboard); break
        case "copyImageAddress": v.triggerWebAction(WebEngineView.CopyImageUrlToClipboard); break
        case "saveImage": v.triggerWebAction(WebEngineView.DownloadImageToDisk); break
        case "playPause": v.triggerWebAction(WebEngineView.ToggleMediaPlayPause); break
        case "mute": v.triggerWebAction(WebEngineView.ToggleMediaMute); break
        case "loop": v.triggerWebAction(WebEngineView.ToggleMediaLoop); break
        case "showControls": v.triggerWebAction(WebEngineView.ToggleMediaControls); break
        case "copyMediaAddress": v.triggerWebAction(WebEngineView.CopyMediaUrlToClipboard); break
        case "saveMedia": v.triggerWebAction(WebEngineView.DownloadMediaToDisk); break
        case "undo": v.triggerWebAction(WebEngineView.Undo); break
        case "redo": v.triggerWebAction(WebEngineView.Redo); break
        case "cut": v.triggerWebAction(WebEngineView.Cut); break
        case "copy": v.triggerWebAction(WebEngineView.Copy); break
        case "paste": v.triggerWebAction(WebEngineView.Paste); break
        case "pasteAsPlainText": v.triggerWebAction(WebEngineView.PasteAndMatchStyle); break
        case "selectAll": v.triggerWebAction(WebEngineView.SelectAll); break
        case "searchSelection": {
            const target = omnibox.resolve(selectedText)
            if (target.toString().length > 0)
                menu.openInNewTab(target)
            break
        }
        case "savePage": v.triggerWebAction(WebEngineView.SavePage); break
        case "screenshot": v.takeScreenshot(false); break
        case "viewSource": v.triggerWebAction(WebEngineView.ViewSource); break
        }
    }

    Connections {
        target: menu.view
        function onContextMenuRequested(request) {
            menu.show(request)
        }
        // The menu lives in the window overlay, so it would stay up over
        // another tab or a new page while acting on this one.
        function onVisibleChanged() {
            if (!menu.view.visible)
                menu.close()
        }
        function onLoadingChanged(request) {
            if (request.status === WebEngineView.LoadStartedStatus)
                menu.close()
        }
    }

    padding: Theme.spacing / 2
    implicitWidth: Theme.tabMaxWidth * 1.2

    background: Rectangle {
        radius: Theme.radius
        color: Theme.surface
        border.color: Theme.border
        border.width: 1
    }

    Component {
        id: itemComponent

        MenuItem {
            id: item
            property string actionId

            implicitHeight: Theme.tabHeight - Theme.spacing
            onTriggered: menu.run(actionId)

            contentItem: Text {
                text: item.text
                color: item.enabled ? Theme.text : Theme.textMuted
                font.pixelSize: Theme.fontSize
                elide: Text.ElideRight
                verticalAlignment: Text.AlignVCenter
                leftPadding: Theme.spacing
            }
            background: Rectangle {
                radius: Theme.radius
                color: item.highlighted ? Theme.surfaceHover : "transparent"
            }
        }
    }

    Component {
        id: separatorComponent

        MenuSeparator {
            topPadding: Theme.spacing / 2
            bottomPadding: Theme.spacing / 2
            contentItem: Rectangle {
                implicitHeight: 1
                color: Theme.border
            }
        }
    }
}
