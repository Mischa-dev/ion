import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// Find in page. Lives inside a BrowserTab, floats over the top-right corner of
// the page. Ctrl+F opens it (with the page's selection, if any), Enter /
// Shift+Enter (or the find-next and find-previous commands, F3 / Shift+F3)
// step through matches, Aa matches case, Esc closes it and clears the
// highlight.
Rectangle {
    id: bar

    required property WebEngineView view

    property int activeMatch: 0
    property int matchCount: 0
    property bool caseSensitive: false
    // Whether the current text has been searched yet, so "no matches" only
    // shows once the page has answered.
    property bool answered: false

    function open() {
        // A short selection on the page is what you want to find, unless
        // you've typed something meanwhile.
        const before = field.text
        view.runJavaScript("String(window.getSelection())", WebEngineScript.ApplicationWorld, text => {
            const selection = String(text ?? "").trim()
            if (bar.visible && field.text === before && selection.length > 0 && selection.length <= 200
                    && selection.indexOf("\n") < 0 && selection !== field.text) {
                field.text = selection
                field.selectAll()
            }
        })
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
        let flags = direction < 0 ? WebEngineView.FindBackward : 0
        if (caseSensitive)
            flags |= WebEngineView.FindCaseSensitively
        view.findText(field.text, flags)
    }
    onCaseSensitiveChanged: search(0)

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
            bar.answered = true
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
                border.width: field.missing || field.activeFocus ? Theme.hairline : 0
                border.color: field.missing ? Theme.danger : Theme.accent
            }

            Text {
                id: counter
                anchors.right: parent.right
                anchors.rightMargin: Theme.spacing * 2
                anchors.verticalCenter: parent.verticalCenter
                text: Basics.findLabel(field.text, bar.activeMatch, bar.matchCount)
                color: field.missing ? Theme.danger : Theme.textMuted
                font.pixelSize: Theme.fontSize - 1
            }

            readonly property bool missing: text.length > 0 && bar.answered && bar.matchCount === 0
            onTextChanged: {
                bar.answered = false
                bar.search(0)
            }
            Keys.onReturnPressed: event => bar.search(event.modifiers & Qt.ShiftModifier ? -1 : 1)
            Keys.onEnterPressed: event => bar.search(event.modifiers & Qt.ShiftModifier ? -1 : 1)
            Keys.onEscapePressed: bar.close()
        }

        IconButton {
            id: caseButton
            glyph: "Aa"
            tip: bar.caseSensitive ? qsTr("Match case: on") : qsTr("Match case")
            active: bar.caseSensitive
            onClicked: bar.caseSensitive = !bar.caseSensitive
        }
        IconButton {
            iconName: "chevron-up"
            tip: qsTr("Previous match")
            enabled: bar.matchCount > 0
            onClicked: bar.search(-1)
        }
        IconButton {
            iconName: "chevron-down"
            tip: qsTr("Next match")
            enabled: bar.matchCount > 0
            onClicked: bar.search(1)
        }
        IconButton {
            iconName: "close"
            tip: qsTr("Close")
            onClicked: bar.close()
        }
    }

    // The find, find-next and find-previous commands (Commands.qml) step
    // through matches, opening the bar first if it is closed.
    function step(direction) {
        if (!visible)
            open()
        else
            search(direction)
    }
}
