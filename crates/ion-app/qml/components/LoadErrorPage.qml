import QtQuick
import QtQuick.Controls
import Ion

// Ion's own page for a load that failed (no connection, unknown site, bad
// certificate), in place of Chromium's. The text comes from `LoadErrors`.
Rectangle {
    id: root

    property string title
    property string detail
    property string pageUrl

    signal retry()

    // Shows the page for a failed load, unless `LoadErrors` says none is due.
    function showFor(info) {
        const lines = LoadErrors.describe(info.errorDomain, info.errorCode,
                                          info.url.toString(), info.errorString)
        if (lines.length < 2) {
            visible = false
            return
        }
        title = lines[0]
        detail = lines[1]
        pageUrl = info.url.toString()
        visible = true
        if (parent.activeFocus)
            takeFocus()
    }

    // Moves keyboard focus here, off the page underneath.
    function takeFocus() {
        retryButton.forceActiveFocus()
    }

    // Above the page content and the new-tab page, below find and prompts.
    z: 6
    visible: false
    color: Theme.background

    // Keeps clicks and scrolling off the page underneath.
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        hoverEnabled: true
        onWheel: wheel => wheel.accepted = true
    }
    // Keys the button doesn't use stop here rather than reach the page.
    Keys.onPressed: event => event.accepted = true
    Keys.onReleased: event => event.accepted = true

    Column {
        anchors.centerIn: parent
        width: Math.min(parent.width - Theme.spacing * 8, Theme.readableWidth)
        spacing: Theme.spacing * 2

        Text {
            width: parent.width
            text: root.title
            color: Theme.text
            font.pixelSize: Theme.titleFontSize
            font.weight: Font.DemiBold
            wrapMode: Text.Wrap
        }
        Text {
            width: parent.width
            text: root.detail
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize + 1
            lineHeight: 1.3
            wrapMode: Text.Wrap
        }
        Text {
            width: parent.width
            text: root.pageUrl
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize - 1
            elide: Text.ElideMiddle
        }
        Item { width: 1; height: Theme.spacing }
        Button {
            id: retryButton
            text: qsTr("Try again")
            implicitHeight: Theme.urlBarHeight
            leftPadding: Theme.spacing * 3
            rightPadding: Theme.spacing * 3
            onClicked: root.retry()

            contentItem: Text {
                text: retryButton.text
                color: Theme.onAccent
                font.pixelSize: Theme.fontSize
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            background: Rectangle {
                radius: Theme.radius
                color: Theme.accent
                opacity: retryButton.down ? 0.85 : retryButton.hovered ? 0.92 : 1
                border.width: retryButton.visualFocus ? Theme.hairline * 2 : 0
                border.color: Theme.text
            }
        }
    }
}
