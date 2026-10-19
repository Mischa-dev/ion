import QtQuick
import QtQuick.Controls
import Ion

// Ion's own page for a load that failed (no connection, unknown site, bad
// certificate) or a page that crashed, drawn over Chromium's. The text comes from `LoadErrors`.
Rectangle {
    id: root

    property string title
    property string detail
    property string pageUrl

    signal retry()
    // The http address offered by "Continue without HTTPS", or "".
    property string insecureUrl: ""
    // "Continue without HTTPS" was picked.
    signal continueInsecure(string url)

    // HTTPS-Only (privacy.httpsOnly = "strict"): the https version of a page
    // failed; offer the http one.
    function showInsecureOffer(info, httpUrl) {
        title = qsTr("This site doesn't offer a secure connection")
        detail = qsTr("Ion tried %1 over HTTPS and it failed. Over plain HTTP, anyone on your network can see and change what you send and receive there.")
            .arg(httpUrl.replace(/^http:\/\//, "").split("/")[0])
        pageUrl = info.url.toString()
        insecureUrl = httpUrl
        visible = true
    }

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
        insecureUrl = ""
        pageUrl = info.url.toString()
        visible = true
    }

    // Shows the page for a tab whose page process ended (crashed, or was
    // killed to free memory). Reload brings the page back.
    function showCrash(status, url) {
        const lines = LoadErrors.describeCrash(status, url.toString())
        if (lines.length < 2)
            return
        title = lines[0]
        detail = lines[1]
        pageUrl = url.toString()
        visible = true
    }

    // Hides the page once the view moves on to another address, so a page
    // that is still loading shows as it renders.
    function leftFor(url) {
        if (visible && url.toString() !== pageUrl)
            visible = false
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
            // Never takes keyboard focus: a hidden item keeps it, so typing
            // would vanish after the retried page loads. Keyboard users
            // retry with Reload (Ctrl+R), which does the same.
            focusPolicy: Qt.NoFocus
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
            }
        }
        Button {
            id: insecureButton
            visible: root.insecureUrl.length > 0
            text: qsTr("Continue without HTTPS")
            focusPolicy: Qt.NoFocus
            implicitHeight: Theme.urlBarHeight
            leftPadding: Theme.spacing * 3
            rightPadding: Theme.spacing * 3
            onClicked: root.continueInsecure(root.insecureUrl)

            contentItem: Text {
                text: insecureButton.text
                color: Theme.text
                font.pixelSize: Theme.fontSize
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            background: Rectangle {
                radius: Theme.radius
                color: insecureButton.down ? Theme.surfaceRaised : insecureButton.hovered ? Theme.surfaceHover : Theme.surfaceRaised
            }
        }
    }
}
