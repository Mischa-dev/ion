import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// What agents may do without asking, each one easy to take back, and what
// they did lately, from the safety model's activity log (docs/SAFETY.md).
// Opened from the palette and the agent sidebar.
Popup {
    id: root

    // How far back the log goes here.
    property int days: 7
    property var rules: []
    property var lines: []
    // `lines` split by day: [{day, lines}], newest first.
    readonly property var groups: {
        const out = []
        for (const line of lines) {
            const day = dayName(line.time)
            if (out.length === 0 || out[out.length - 1].day !== day)
                out.push({ day: day, lines: [] })
            out[out.length - 1].lines.push(line)
        }
        return out
    }

    function refresh() {
        rules = JSON.parse(Safety.agentRulesJson() || "[]")
        lines = JSON.parse(Safety.activityJson(days) || "[]")
    }

    function show() {
        refresh()
        open()
    }

    function dayName(time) {
        const date = new Date(time * 1000)
        const today = new Date()
        const start = d => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()
        const ago = Math.round((start(today) - start(date)) / 86400000)
        if (ago === 0)
            return qsTr("Today")
        if (ago === 1)
            return qsTr("Yesterday")
        return date.toLocaleDateString(Qt.locale(), "dddd d MMMM")
    }

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Theme.urlBarHeight * 3
    width: Math.min(Theme.paletteWidth, parent.width - Theme.spacing * 4)
    padding: Theme.spacing * 3
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    onOpened: Qt.callLater(() => content.forceActiveFocus())

    Overlay.modal: Rectangle {
        color: Qt.rgba(Theme.background.r, Theme.background.g, Theme.background.b, Theme.scrimOpacity)
    }

    background: Rectangle {
        radius: Theme.radius
        color: Theme.surface
        border.width: 1
        border.color: Theme.border
    }

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
    }

    component Heading: Text {
        Layout.fillWidth: true
        Layout.topMargin: Theme.spacing
        color: Theme.textMuted
        font.pixelSize: Theme.fontSize - 1
        font.bold: true
    }

    contentItem: ColumnLayout {
        id: content
        spacing: Theme.spacing
        focus: true
        Keys.onEscapePressed: root.close()

        Text {
            Layout.fillWidth: true
            text: qsTr("Agent activity")
            color: Theme.text
            font.pixelSize: Theme.fontSize + 2
            font.bold: true
        }

        Flickable {
            id: scroller
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(body.implicitHeight, root.parent ? root.parent.height * 0.65 : 480)
            contentWidth: width
            contentHeight: body.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            interactive: contentHeight > height

            ColumnLayout {
                id: body
                width: scroller.width
                spacing: Theme.spacing

                Heading { text: qsTr("Without asking") }
                Text {
                    Layout.fillWidth: true
                    visible: root.rules.length === 0
                    text: qsTr("Agents ask before everything. When you choose Allow on a site, it shows here, so you can take it back.")
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSize
                    wrapMode: Text.Wrap
                }
                Repeater {
                    model: root.rules
                    delegate: Rectangle {
                        id: ruleRow
                        required property var modelData
                        Layout.fillWidth: true
                        implicitHeight: ruleLayout.implicitHeight + Theme.spacing * 2
                        radius: Theme.radius
                        color: Theme.surfaceRaised

                        RowLayout {
                            id: ruleLayout
                            anchors.fill: parent
                            anchors.margins: Theme.spacing
                            anchors.leftMargin: Theme.spacing * 2
                            spacing: Theme.spacing * 2

                            Text {
                                text: ruleRow.modelData.agentName
                                color: Theme.text
                                font.pixelSize: Theme.fontSize
                                font.bold: true
                            }
                            Text {
                                Layout.fillWidth: true
                                text: ruleRow.modelData.text
                                color: Theme.text
                                font.pixelSize: Theme.fontSize
                                elide: Text.ElideRight
                            }
                            DialogButton {
                                implicitHeight: Theme.urlBarHeight - Theme.spacing
                                text: qsTr("Take back")
                                onClicked: {
                                    Safety.forgetAgentRule(ruleRow.modelData.rule)
                                    root.refresh()
                                }
                            }
                        }
                    }
                }

                Heading {
                    text: qsTr("Last %1 days").arg(root.days)
                }
                Text {
                    Layout.fillWidth: true
                    visible: root.lines.length === 0
                    text: qsTr("Nothing yet.")
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSize
                }
                Repeater {
                    model: root.groups
                    delegate: ColumnLayout {
                        id: group
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: Theme.spacing / 2

                        Text {
                            Layout.topMargin: Theme.spacing
                            text: group.modelData.day
                            color: Theme.text
                            font.pixelSize: Theme.fontSize
                        }
                        Repeater {
                            model: group.modelData.lines
                            delegate: RowLayout {
                                id: line
                                required property var modelData
                                Layout.fillWidth: true
                                spacing: Theme.spacing * 2

                                Text {
                                    text: new Date(line.modelData.time * 1000)
                                        .toLocaleTimeString(Qt.locale(), Locale.ShortFormat)
                                    color: Theme.textMuted
                                    font.pixelSize: Theme.fontSize - 1
                                    font.features: { "tnum": 1 }
                                }
                                Text {
                                    Layout.preferredWidth: Theme.iconSize
                                    horizontalAlignment: Text.AlignHCenter
                                    text: {
                                        switch (line.modelData.mark) {
                                        case "done": return "✓"
                                        case "failed": return "✕"
                                        case "allowed": return "✓"
                                        case "refused": return "⊘"
                                        }
                                        return "·"
                                    }
                                    color: {
                                        switch (line.modelData.mark) {
                                        case "failed": return Theme.danger
                                        case "allowed": return Theme.accent
                                        case "refused": return Theme.warning
                                        }
                                        return Theme.textMuted
                                    }
                                    font.pixelSize: Theme.fontSize - 1
                                }
                                Text {
                                    Layout.fillWidth: true
                                    text: line.modelData.text
                                    color: Theme.text
                                    font.pixelSize: Theme.fontSize - 1
                                    elide: Text.ElideRight
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
