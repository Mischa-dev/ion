import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// The ad blocker's panel: what was blocked here, the switch for this site,
// and the state of the filter lists.
Popup {
    id: popup

    property var view   // BrowserTab of the current tab, may be null

    readonly property url pageUrl: view?.url ?? ""
    readonly property string site: Adblock.siteOf(pageUrl)
    readonly property bool siteEnabled: {
        Adblock.revision
        return Adblock.isEnabledOn(pageUrl)
    }
    readonly property int blocked: {
        Adblock.revision
        return Adblock.blockedOn(pageUrl)
    }

    width: Theme.tabMaxWidth + Theme.spacing * 14
    padding: Theme.spacing * 2
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent

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

    // "3 hours ago" for a millisecond timestamp.
    function ago(ms) {
        const minutes = Math.floor((Date.now() - ms) / 60000)
        const hours = Math.floor(minutes / 60)
        const days = Math.floor(hours / 24)
        if (minutes < 1)
            return qsTr("just now")
        if (minutes < 60)
            return minutes === 1 ? qsTr("a minute ago") : qsTr("%1 minutes ago").arg(minutes)
        if (hours < 24)
            return hours === 1 ? qsTr("an hour ago") : qsTr("%1 hours ago").arg(hours)
        return days === 1 ? qsTr("yesterday") : qsTr("%1 days ago").arg(days)
    }

    component Caption: Label {
        color: Theme.textMuted
        font.pixelSize: Theme.fontSize - 1
        wrapMode: Text.Wrap
        Layout.fillWidth: true
    }

    component ThemedSwitch: Switch {
        id: toggle
        focusPolicy: Qt.NoFocus
        padding: 0
        implicitWidth: indicator.implicitWidth
        implicitHeight: indicator.implicitHeight
        indicator: Rectangle {
            implicitWidth: Theme.iconSize * 2
            implicitHeight: Theme.iconSize + Theme.spacing / 2
            radius: height / 2
            color: toggle.checked ? Theme.accent : Theme.surfaceHover
            Behavior on color { ColorAnimation { duration: Theme.animationMs } }

            Rectangle {
                width: parent.height - Theme.spacing / 2
                height: width
                radius: width / 2
                y: (parent.height - height) / 2
                x: toggle.checked ? parent.width - width - y : y
                color: toggle.checked ? Theme.background : Theme.text
                Behavior on x { NumberAnimation { duration: Theme.animationMs } }
            }
        }
        contentItem: Item {}
    }

    contentItem: ColumnLayout {
        spacing: Theme.spacing * 1.5

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing

            Label {
                Layout.fillWidth: true
                text: qsTr("Ad blocker")
                color: Theme.text
                font.pixelSize: Theme.fontSize
                font.weight: Font.DemiBold
            }
            ThemedSwitch {
                id: globalSwitch
                checked: Adblock.enabled
                onToggled: {
                    // Saved to overrides.toml; Adblock follows the config a
                    // moment later. On failure, show the real state again.
                    if (Config.set("adblock.enable", checked) === "")
                        popup.view?.reloadPage()
                    else
                        checked = Adblock.enabled
                }
                // Toggling breaks the binding, so follow later config reloads by hand.
                Connections {
                    target: Adblock
                    function onEnabledChanged() { globalSwitch.checked = Adblock.enabled }
                }
                ToolTip.visible: hovered
                ToolTip.delay: 600
                ToolTip.text: checked ? qsTr("Turn off everywhere") : qsTr("Turn on")
            }
        }

        ColumnLayout {
            visible: popup.site.length > 0 && Adblock.enabled
            Layout.fillWidth: true
            spacing: Theme.spacing

            Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.border }

            Label {
                Layout.fillWidth: true
                text: popup.site
                color: Theme.text
                font.pixelSize: Theme.fontSize
                elide: Text.ElideMiddle
            }

            Label {
                Layout.fillWidth: true
                text: popup.siteEnabled ? qsTr("%n blocked on this page", "", popup.blocked)
                                        : qsTr("Not blocking on this site")
                color: popup.siteEnabled ? Theme.accent : Theme.textMuted
                font.pixelSize: Theme.fontSize + 3
                font.weight: Font.DemiBold
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spacing

                Label {
                    Layout.fillWidth: true
                    text: qsTr("Block ads and trackers on this site")
                    color: Theme.text
                    font.pixelSize: Theme.fontSize
                    wrapMode: Text.Wrap
                }
                ThemedSwitch {
                    checked: popup.siteEnabled
                    onToggled: {
                        const changed = Adblock.setEnabledOn(popup.pageUrl, checked)
                        // Toggling breaks the binding; restore it so a rejected
                        // switch (e.g. on github.io) snaps back.
                        checked = Qt.binding(() => popup.siteEnabled)
                        if (changed)
                            popup.view?.reloadPage()
                    }
                }
            }
        }

        Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.border }

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing

            Caption {
                text: Adblock.updating ? qsTr("Updating filter lists…")
                    : Adblock.lastUpdated > 0 ? qsTr("Filter lists updated %1").arg(popup.ago(Adblock.lastUpdated))
                    : qsTr("Filter lists not downloaded yet")
            }
            ToolButton {
                id: updateButton
                enabled: !Adblock.updating
                focusPolicy: Qt.NoFocus
                onClicked: Adblock.updateLists()
                contentItem: Text {
                    text: qsTr("Update")
                    color: updateButton.enabled ? Theme.accent : Theme.textMuted
                    font.pixelSize: Theme.fontSize - 1
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                background: Rectangle {
                    radius: Theme.radius
                    color: updateButton.hovered ? Theme.surfaceHover : "transparent"
                }
            }
        }

        Caption {
            visible: Adblock.error.length > 0
            text: Adblock.error
        }

        Caption {
            text: qsTr("%n blocked since Ion started", "", Adblock.totalBlocked)
        }
    }
}
