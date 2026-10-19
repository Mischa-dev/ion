import QtQuick
import QtQuick.Controls
import Ion

// The card shown when the pointer rests on a tab: its full title, its site,
// and a picture of the page from when it was last on screen (Chrome's hover
// cards, Edge's and Vivaldi's previews). Opens below the tab in the top
// strip and to its right in the sidebar. The tab sets `wanted` while hovered.
Popup {
    id: card

    property var view: null
    property string title
    property string url
    property bool suspended: false
    property bool current: false
    // Open to the right of the tab (the sidebar) instead of below it.
    property bool beside: false
    property bool wanted: false

    readonly property string site: {
        const match = url.match(/^[a-z][a-z0-9+.-]*:\/\/([^\/?#]*)/)
        return match && match[1].length > 0 ? match[1].replace(/^www\./, "") : url
    }
    // The current tab is on screen already; others show their picture.
    readonly property bool showPicture: !current && (view?.snapshot.toString().length ?? 0) > 0
    readonly property string state: suspended ? qsTr("Asleep")
        : view?.audioMuted ? qsTr("Muted")
        : view?.recentlyAudible ? qsTr("Playing audio")
        : view?.needsAttention ? qsTr("Waiting for you")
        : ""

    x: beside ? parent.width + Theme.spacing : 0
    y: beside ? 0 : parent.height + Theme.spacing
    width: Theme.previewWidth + padding * 2
    padding: Theme.spacing
    margins: Theme.spacing
    closePolicy: Popup.NoAutoClose
    focus: false

    Timer {
        interval: Theme.hoverDelayMs * 2
        running: card.wanted && !card.opened
        onTriggered: card.open()
    }
    onWantedChanged: if (!wanted) close()

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
    }

    background: Rectangle {
        radius: Theme.radius
        color: Theme.surface
        border.color: Theme.border
        border.width: Theme.hairline
    }

    contentItem: Column {
        spacing: Theme.spacing

        Text {
            width: parent.width
            leftPadding: Theme.spacing / 2
            rightPadding: Theme.spacing / 2
            topPadding: Theme.spacing / 2
            text: card.title.length > 0 ? card.title : qsTr("New Tab")
            color: Theme.text
            font.pixelSize: Theme.fontSize
            font.bold: true
            wrapMode: Text.Wrap
            maximumLineCount: 2
            elide: Text.ElideRight
            textFormat: Text.PlainText
        }

        Text {
            width: parent.width
            leftPadding: Theme.spacing / 2
            rightPadding: Theme.spacing / 2
            bottomPadding: card.showPicture ? 0 : Theme.spacing / 2
            visible: text.length > 0
            text: card.state.length > 0 ? card.site + " · " + card.state : card.site
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize - 1
            elide: Text.ElideMiddle
            textFormat: Text.PlainText
        }

        Rectangle {
            visible: card.showPicture
            width: parent.width
            height: Math.round(width * 5 / 8)
            radius: Theme.radius / 2
            color: Theme.background
            clip: true

            Image {
                anchors.fill: parent
                source: card.showPicture ? card.view.snapshot : ""
                fillMode: Image.PreserveAspectCrop
                verticalAlignment: Image.AlignTop
                asynchronous: true
                cache: false
                opacity: card.suspended ? Theme.dimmedOpacity : 1
            }
        }
    }
}
