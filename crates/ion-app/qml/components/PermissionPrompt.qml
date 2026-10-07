import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// Ion's own prompt for camera, microphone, location, notifications and the
// other site permissions. Lives inside a BrowserTab, below the toolbar on the
// left like a doorhanger. Requests queue up and are asked one at a time.
// Ion's safety model (Safety, docs/SAFETY.md) decides, words the prompt and
// remembers answers; the engine's own permission store is off (Main.qml).
Rectangle {
    id: prompt

    required property WebEngineView view
    // The tab's id in the `Tabs` model; Safety scopes answers to it.
    required property int tabId

    // Pending requests, oldest first: { permission, text, choices, generation }
    // with the prompt Safety built for each.
    property var queue: []
    readonly property var current: queue.length > 0 ? queue[0] : null
    readonly property string message: current ? current.text : ""
    // Allow stays disabled for a moment after each new request shows, so a
    // click aimed at the page can't grant it by accident. The delay starts
    // again whenever the prompt comes back into view: its tab is shown, or
    // the window is raised or restored.
    property bool armed: false
    readonly property bool onScreen: view.visible && Window.active

    function rearm() {
        armed = false
        armTimer.stop()
        if (current && onScreen)
            armTimer.start()
    }
    onCurrentChanged: rearm()
    onOnScreenChanged: rearm()

    Timer {
        id: armTimer
        interval: Theme.promptArmMs
        onTriggered: prompt.armed = true
    }

    // Settle `permission` if Safety already knows the answer. Returns the
    // prompt to show otherwise, or null once settled.
    function settle(permission) {
        const decision = Safety.siteDecision(permission.permissionType, permission.origin, tabId)
        if (decision === "allow") {
            permission.grant()
            return null
        }
        if (decision !== "ask") {
            permission.deny()
            return null
        }
        const json = Safety.sitePrompt(permission.permissionType, permission.origin, tabId)
        if (json.length === 0) {
            permission.deny()
            return null
        }
        const shown = JSON.parse(json)
        shown.permission = permission
        return shown
    }

    function enqueue(permission) {
        const shown = settle(permission)
        if (shown)
            queue = queue.concat([shown])
    }

    function answer(choice) {
        if (!current || (choice !== "deny" && !armed))
            return
        const p = current.permission
        if (Safety.answerSite(p.permissionType, p.origin, tabId, current.generation, choice))
            p.grant()
        else
            p.deny()
        // An answer that was remembered may settle requests still waiting.
        queue = queue.slice(1).map(item => settle(item.permission)).filter(item => item !== null)
    }

    // A new page cancels whatever the old one asked for. The engine drops those
    // requests itself; denying them here would store a "Block" the user never
    // chose for persistent kinds like notifications or location.
    function dropAll() {
        queue = []
    }

    visible: current !== null
    z: 10
    anchors.top: parent.top
    anchors.left: parent.left
    anchors.margins: Theme.spacing * 2
    width: Math.min(parent.width - Theme.spacing * 4, Theme.tabMaxWidth * 1.8)
    height: content.implicitHeight + Theme.spacing * 4
    radius: Theme.radius
    color: Theme.surface
    border.color: Theme.border
    border.width: 1

    Connections {
        target: prompt.view
        function onPermissionRequested(permission) {
            prompt.enqueue(permission)
        }
        function onLoadingChanged(request) {
            if (request.status === WebEngineView.LoadStartedStatus)
                prompt.dropAll()
        }
    }

    ColumnLayout {
        id: content
        anchors.fill: parent
        anchors.margins: Theme.spacing * 2
        spacing: Theme.spacing * 2

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing * 2

            Text {
                text: prompt.current ? Basics.permissionGlyph(prompt.current.permission.permissionType) : ""
                font.pixelSize: Theme.fontSize + 7
                color: Theme.text
            }
            Text {
                Layout.fillWidth: true
                text: prompt.message
                color: Theme.text
                font.pixelSize: Theme.fontSize
                wrapMode: Text.Wrap
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spacing

            // Safety's choices, primary last: Block, Allow this time, Allow.
            Repeater {
                model: prompt.current ? prompt.current.choices : []
                delegate: PromptButton {
                    required property var modelData
                    required property int index
                    text: modelData.label
                    primary: index === prompt.current.choices.length - 1 && modelData.id !== "deny"
                    enabled: modelData.id === "deny" || prompt.armed
                    onClicked: prompt.answer(modelData.id)
                }
            }
        }
    }

    component PromptButton: Button {
        id: button
        property bool primary: false

        implicitHeight: Theme.urlBarHeight
        leftPadding: Theme.spacing * 3
        rightPadding: Theme.spacing * 3
        focusPolicy: Qt.NoFocus

        contentItem: Text {
            text: button.text
            color: button.primary ? Theme.background : Theme.text
            font.pixelSize: Theme.fontSize
            font.bold: button.primary
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
        background: Rectangle {
            radius: Theme.radius
            color: button.primary ? Theme.accent : button.hovered ? Theme.surfaceHover : Theme.surfaceRaised
            opacity: !button.enabled ? 0.5 : button.primary && button.down ? 0.8 : 1
        }
    }
}
