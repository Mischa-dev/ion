import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// The side panel beside the page: notes, and web apps kept open there (chat,
// music, mail…), like Vivaldi's web panels. A strip at the top switches
// between them; + adds the current page. Each web panel's page loads the
// first time it is shown and then stays alive, so music keeps playing and
// chats keep notifying while the panel is closed. Links that open new
// windows open as tabs. The left edge drags to resize. State lives in the
// `Panels` singleton.
Item {
    id: root

    // The main window: profile, currentView and openTab(url, activate).
    required property var browser

    readonly property string open: Panels.open
    // What Ctrl+Shift+L reopens.
    property string lastOpen: "notes"
    onOpenChanged: if (open.length > 0) lastOpen = open

    // Width while dragging; saved when the drag ends.
    property int dragWidth: -1
    readonly property int panelWidth: dragWidth > 0 ? dragWidth : Panels.width

    readonly property var webPanels: { Panels.revision; return Panels.webPanels() }
    readonly property var current: webPanels.find(p => p.id === open) ?? null

    visible: open.length > 0
    implicitWidth: visible ? panelWidth : 0

    Rectangle {
        anchors.fill: parent
        color: Theme.surface
    }
    Rectangle {
        anchors.left: parent.left
        width: Theme.hairline
        height: parent.height
        color: Theme.border
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.hairline
        spacing: 0

        // The strip: notes, each web panel, add, then this panel's actions.
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: Theme.spacing
            spacing: Theme.spacing / 2

            PanelButton {
                iconName: "note"
                tip: qsTr("Notes")
                selected: root.open === "notes"
                onClicked: Panels.show("notes")
            }

            Repeater {
                model: root.webPanels

                delegate: PanelButton {
                    required property var modelData
                    glyph: modelData.label.charAt(0).toUpperCase()
                    iconPage: modelData.url
                    tip: modelData.label
                    selected: root.open === modelData.id
                    onClicked: Panels.show(modelData.id)
                    onMenuRequested: {
                        panelMenu.panel = modelData
                        panelMenu.popup()
                    }
                }
            }

            IconButton {
                iconName: "plus"
                tip: qsTr("Add this page to the side panel")
                enabled: !!root.browser.currentView
                    && /^https?:/.test(root.browser.currentView.url.toString())
                onClicked: {
                    const view = root.browser.currentView
                    const id = Panels.add(view.url.toString(), view.title)
                    if (id.length > 0)
                        Panels.show(id)
                }
            }

            Item { Layout.fillWidth: true }

            IconButton {
                iconName: "external"
                tip: qsTr("Open in a tab")
                visible: root.current !== null
                onClicked: root.browser.openTab(root.current.url)
            }
            IconButton {
                iconName: "close"
                tip: qsTr("Close the side panel")
                onClicked: Panels.show("")
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.hairline
            color: Theme.border
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            NotesPanel {
                anchors.fill: parent
                anchors.margins: Theme.spacing
                visible: root.open === "notes"
                browser: root.browser
            }

            Repeater {
                model: root.webPanels

                delegate: Loader {
                    id: loader

                    required property var modelData
                    readonly property bool shown: root.open === modelData.id

                    anchors.fill: parent
                    visible: shown
                    // Loaded on first show, then kept. Not before the window
                    // has its profile: a view must never get a null one.
                    active: !!root.browser.profile && (shown || item !== null)

                    sourceComponent: WebEngineView {
                        profile: root.browser.profile
                        url: loader.modelData.url
                        settings.fullScreenSupportEnabled: false
                        onNewWindowRequested: request => root.browser.openTab(request.requestedUrl)
                    }
                }
            }
        }
    }

    // Drag the left edge to resize.
    MouseArea {
        anchors.left: parent.left
        anchors.leftMargin: -Theme.spacing / 2
        width: Theme.spacing
        height: parent.height
        cursorShape: Qt.SizeHorCursor
        property real startX
        property int startWidth
        onPressed: mouse => {
            startX = mapToItem(null, mouse.x, 0).x
            startWidth = root.panelWidth
        }
        onPositionChanged: mouse => {
            if (pressed)
                root.dragWidth = Math.max(Theme.agentSidebarMinWidth,
                    startWidth + startX - mapToItem(null, mouse.x, 0).x)
        }
        onReleased: {
            Panels.setWidth(root.dragWidth)
            root.dragWidth = -1
        }
    }

    IonMenu {
        id: panelMenu
        property var panel: null

        IonMenuItem {
            text: qsTr("Open in a tab")
            onTriggered: root.browser.openTab(panelMenu.panel.url)
        }
        IonMenuItem {
            text: qsTr("Remove from side panel")
            onTriggered: Panels.remove(panelMenu.panel.id)
        }
    }
}
