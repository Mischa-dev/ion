import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// Find in page. Lives inside a BrowserTab, floats over the top-right corner of
// the page. Ctrl+F opens it, Enter / Shift+Enter (or F3 / Shift+F3) step
// through matches, Esc closes it and clears the highlight.
Rectangle {
    id: bar

    required property WebEngineView view

    property int activeMatch: 0
    property int matchCount: 0

    function open() {
        visible = true
        field.forceActiveFocus()
        field.selectAll()
        if (field.text.length > 0)
            search(0)
    }

    function close(refocusPage) {
        visible = false
        view.findText("")
        matchCount = 0
        activeMatch = 0
        if (refocusPage !== false)
            view.forceActiveFocus()
    }

    // direction: 0 = search again from the start, 1 = next, -1 = previous.
    function search(direction) {
        if (field.text.length === 0) {
            view.findText("")
            matchCount = 0
            activeMatch = 0
            return
        }
        view.findText(field.text, direction < 0 ? WebEngineView.FindBackward : 0)
    }

    visible: false
    z: 10
    anchors.top: parent.top
    anchors.right: parent.right
    anchors.margins: Theme.spacing * 2
    width: Math.min(parent.width - Theme.spacing * 4, Theme.tabMaxWidth * 1.8)
    height: Theme.urlBarHeight + Theme.spacing * 2
    radius: Theme.radius
    color: Theme.surface
    border.color: Theme.border
    border.width: 1

    Connections {
        target: bar.view
        function onFindTextFinished(result) {
            bar.matchCount = result.numberOfMatches
            bar.activeMatch = result.activeMatch
        }
        function onUrlChanged() {
            if (bar.visible)
                bar.close(false)
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: Theme.spacing
        spacing: Theme.spacing / 2

        TextField {
            id: field
            Layout.fillWidth: true
            implicitHeight: Theme.urlBarHeight
            color: Theme.text
            placeholderText: qsTr("Find in page")
            placeholderTextColor: Theme.textMuted
            selectionColor: Theme.accent
            selectedTextColor: Theme.background
            font.pixelSize: Theme.fontSize
            leftPadding: Theme.spacing * 2
            rightPadding: counter.implicitWidth + Theme.spacing * 3
            verticalAlignment: TextInput.AlignVCenter
            selectByMouse: true

            background: Rectangle {
                radius: Theme.radius
                color: Theme.surfaceRaised
                border.width: field.activeFocus ? 1 : 0
                border.color: Theme.accent
            }

            Text {
                id: counter
                anchors.right: parent.right
                anchors.rightMargin: Theme.spacing * 2
                anchors.verticalCenter: parent.verticalCenter
                text: Basics.findLabel(field.text, bar.activeMatch, bar.matchCount)
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize - 1
            }

            onTextChanged: bar.search(0)
            Keys.onReturnPressed: event => bar.search(event.modifiers & Qt.ShiftModifier ? -1 : 1)
            Keys.onEnterPressed: event => bar.search(event.modifiers & Qt.ShiftModifier ? -1 : 1)
            Keys.onEscapePressed: bar.close()
        }

        IconButton {
            glyph: "↑"
            tip: qsTr("Previous match")
            enabled: bar.matchCount > 0
            onClicked: bar.search(-1)
        }
        IconButton {
            glyph: "↓"
            tip: qsTr("Next match")
            enabled: bar.matchCount > 0
            onClicked: bar.search(1)
        }
        IconButton {
            glyph: "×"
            tip: qsTr("Close")
            onClicked: bar.close()
        }
    }

    // Only the visible tab's bar takes the shortcuts.
    Shortcut {
        sequences: [StandardKey.Find]
        enabled: bar.view.visible
        onActivated: bar.open()
    }
    Shortcut {
        sequences: [StandardKey.FindNext]
        enabled: bar.view.visible && bar.visible
        onActivated: bar.search(1)
    }
    Shortcut {
        sequences: [StandardKey.FindPrevious]
        enabled: bar.view.visible && bar.visible
        onActivated: bar.search(-1)
    }
}
