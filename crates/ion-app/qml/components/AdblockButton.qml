import QtQuick
import QtQuick.Controls
import QtQuick.Shapes
import Ion

// Shield next to the URL bar: lit while the ad blocker works on this page,
// with the number of blocked requests. Click for the per-site switch.
ToolButton {
    id: control

    property var view   // BrowserTab of the current tab, may be null

    readonly property url pageUrl: view?.url ?? ""
    readonly property bool hasSite: Adblock.siteOf(pageUrl).length > 0
    readonly property bool active: {
        Adblock.revision
        return Adblock.enabled && Adblock.ready && Adblock.isEnabledOn(pageUrl)
    }
    readonly property int blocked: {
        Adblock.revision
        return Adblock.blockedOn(pageUrl)
    }

    implicitWidth: Theme.urlBarHeight
    implicitHeight: Theme.urlBarHeight
    focusPolicy: Qt.NoFocus
    enabled: view !== null

    onClicked: popup.opened ? popup.close() : popup.open()

    contentItem: Item {
        Shape {
            id: shield
            anchors.centerIn: parent
            width: Theme.iconSize
            height: Theme.iconSize
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeColor: control.active ? Theme.accent : Theme.textMuted
                strokeWidth: 1.5
                fillColor: control.active ? Theme.accent : "transparent"
                joinStyle: ShapePath.RoundJoin
                scale: Qt.size(shield.width / 16, shield.height / 16)
                PathSvg { path: "M 8 1 L 14 3.5 L 14 8 C 14 11.5 11.4 14 8 15 C 4.6 14 2 11.5 2 8 L 2 3.5 Z" }
            }
        }

        // Count badge, bottom right of the shield.
        Rectangle {
            visible: control.active && control.blocked > 0
            anchors.left: shield.horizontalCenter
            anchors.top: shield.verticalCenter
            width: Math.max(height, badgeText.implicitWidth + Theme.spacing)
            height: badgeText.implicitHeight
            radius: height / 2
            color: Theme.text

            Text {
                id: badgeText
                anchors.centerIn: parent
                text: control.blocked > 99 ? "99+" : control.blocked
                color: Theme.background
                font.pixelSize: Theme.fontSize - 4
                font.weight: Font.DemiBold
            }
        }
    }

    background: Rectangle {
        radius: Theme.radius
        color: control.down || popup.opened ? Theme.surfaceRaised
             : control.hovered ? Theme.surfaceHover : "transparent"
    }

    ToolTip.visible: hovered && !popup.opened
    ToolTip.delay: 600
    ToolTip.text: !Adblock.enabled ? qsTr("Ad blocker is off")
                : !control.hasSite ? qsTr("Ad blocker")
                : !control.active && Adblock.ready ? qsTr("Ad blocker is off on %1").arg(Adblock.siteOf(control.pageUrl))
                : qsTr("%n blocked on this page", "", control.blocked)

    AdblockPopup {
        id: popup
        view: control.view
        y: control.height + Theme.spacing / 2
        x: control.width - width
    }
}
