import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// Key next to the URL bar, shown on pages with a login form or saved logins.
// After a sign-in it opens by itself to offer saving (or updating) the
// password, and after an address form to offer saving the address;
// otherwise it lists the site's saved logins to copy from.
ToolButton {
    id: control

    property var view   // BrowserTab of the current tab, may be null

    // "Manage passwords": the manager, searching for `query`.
    signal manageRequested(string query)

    readonly property var fill: view?.passwords ?? null
    readonly property var prompt: fill?.prompt ?? null
    readonly property bool addressPrompt: prompt?.kind === "address"
    readonly property string pageUrl: (view?.url ?? "").toString()
    readonly property string host: Passwords.host(pageUrl)
    // Saved logins for this page; reading revision refreshes after edits.
    readonly property var logins: {
        Passwords.revision
        return (fill?.formOnPage || prompt) && host.length > 0 ? JSON.parse(Passwords.matches(pageUrl)) : []
    }

    visible: prompt !== null || (fill?.formOnPage ?? false) || logins.length > 0
    implicitWidth: Theme.urlBarHeight
    implicitHeight: Theme.urlBarHeight
    focusPolicy: Qt.NoFocus

    onClicked: popup.opened ? popup.close() : popup.open()

    // A new offer on the current tab opens the panel.
    Connections {
        target: control.fill
        function onPromptRaised() { popup.open() }
    }
    onViewChanged: {
        popup.close()
        // A tab with an offer from while it was in the background shows it now.
        if (prompt)
            Qt.callLater(popup.open)
    }

    contentItem: Item {
        Icon {
            anchors.centerIn: parent
            name: control.addressPrompt ? "home" : "key"
            color: control.prompt ? Theme.accent : Theme.text
        }
        // A dot while an offer waits.
        Rectangle {
            visible: control.prompt !== null && !popup.opened
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: Theme.spacing / 2
            width: Theme.spacing + 1
            height: width
            radius: width / 2
            color: Theme.accent
        }
    }

    background: Rectangle {
        radius: Theme.radius
        color: control.down || popup.opened ? Theme.surfaceRaised
             : control.hovered ? Theme.surfaceHover : "transparent"
    }

    ToolTip.visible: hovered && !popup.opened
    ToolTip.delay: 600
    ToolTip.text: control.addressPrompt ? qsTr("Save address")
                : control.prompt ? qsTr("Save password") : qsTr("Passwords for %1").arg(control.host)

    Popup {
        id: popup

        y: control.height + Theme.spacing / 2
        x: control.width - width
        width: Theme.tabMaxWidth + Theme.spacing * 22
        padding: Theme.spacing * 2
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent

        // The offer being edited: the username can be corrected before saving.
        property bool reveal: false
        onOpened: {
            reveal = false
            username.text = control.prompt?.username ?? ""
            password.text = control.prompt?.password ?? ""
        }

        function save() {
            const offer = control.prompt
            if (!offer)
                return
            if (offer.kind === "update" && username.text === offer.username)
                Passwords.updatePassword(offer.id, password.text)
            else
                Passwords.save(offer.url, username.text, password.text)
            control.fill.dismissPrompt()
            close()
        }

        background: Rectangle {
            radius: Theme.radius
            color: Theme.surface
            border.width: Theme.hairline
            border.color: Theme.border
        }
        enter: Transition {
            NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
        }
        exit: Transition {
            NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
        }

        component Caption: Label {
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize - 1
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }

        contentItem: ColumnLayout {
            spacing: Theme.spacing * 1.5

            // Save an address.
            ColumnLayout {
                visible: control.addressPrompt
                Layout.fillWidth: true
                spacing: Theme.spacing * 1.5

                Label {
                    Layout.fillWidth: true
                    text: qsTr("Save this address?")
                    color: Theme.text
                    font.pixelSize: Theme.fontSize + 1
                    font.weight: Font.DemiBold
                }
                Label {
                    Layout.fillWidth: true
                    text: {
                        const a = control.prompt?.address
                        if (!a)
                            return ""
                        const place = [a.city, a.region, a.postalCode].filter(p => p.length > 0).join(" ")
                        return [a.name, a.organization, a.street, place, a.country, a.email, a.phone]
                            .filter(p => p && p.length > 0).join("\n")
                    }
                    textFormat: Text.PlainText
                    color: Theme.text
                    font.pixelSize: Theme.fontSize
                    wrapMode: Text.Wrap
                }
                Caption {
                    text: qsTr("Ion fills it in on forms that ask for an address. Saved encrypted, with your passwords.")
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacing
                    Item { Layout.fillWidth: true }
                    DialogButton {
                        text: qsTr("Not now")
                        onClicked: {
                            control.fill.dismissPrompt()
                            popup.close()
                        }
                    }
                    DialogButton {
                        primary: true
                        text: qsTr("Save")
                        onClicked: {
                            Passwords.saveAddress(JSON.stringify(control.prompt.address))
                            control.fill.dismissPrompt()
                            popup.close()
                        }
                    }
                }
            }

            // Save or update offer.
            ColumnLayout {
                visible: control.prompt !== null && !control.addressPrompt
                Layout.fillWidth: true
                spacing: Theme.spacing * 1.5

                Label {
                    Layout.fillWidth: true
                    text: control.prompt?.kind === "update"
                          ? qsTr("Update the password for %1?").arg(Passwords.host(control.prompt.url))
                          : qsTr("Save the password for %1?").arg(Passwords.host(control.prompt?.url ?? ""))
                    color: Theme.text
                    font.pixelSize: Theme.fontSize + 1
                    font.weight: Font.DemiBold
                    wrapMode: Text.Wrap
                }
                IonTextField {
                    id: username
                    Layout.fillWidth: true
                    placeholderText: qsTr("Username")
                    onAccepted: popup.save()
                }
                IonTextField {
                    id: password
                    Layout.fillWidth: true
                    echoMode: popup.reveal ? TextInput.Normal : TextInput.Password
                    font.family: popup.reveal ? "monospace" : undefined
                    rightPadding: Theme.urlBarHeight + Theme.spacing
                    onAccepted: popup.save()

                    IconButton {
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        iconName: popup.reveal ? "eye-off" : "eye"
                        tip: popup.reveal ? qsTr("Hide password") : qsTr("Show password")
                        onClicked: popup.reveal = !popup.reveal
                    }
                }
                Caption {
                    text: Passwords.storeIsKeyring()
                          ? qsTr("Encrypted, with the key in your %1.").arg(Passwords.storeName())
                          : qsTr("No system keyring answered, so the key is kept in a private file.")
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacing

                    DialogButton {
                        visible: control.prompt?.kind === "save"
                        text: qsTr("Never for this site")
                        onClicked: {
                            Passwords.neverSave(control.prompt.url)
                            control.fill.dismissPrompt()
                            popup.close()
                        }
                    }
                    Item { Layout.fillWidth: true }
                    DialogButton {
                        text: qsTr("Not now")
                        onClicked: {
                            control.fill.dismissPrompt()
                            popup.close()
                        }
                    }
                    DialogButton {
                        primary: true
                        enabled: password.text.length > 0
                        text: control.prompt?.kind === "update" && username.text === control.prompt.username
                              ? qsTr("Update") : qsTr("Save")
                        onClicked: popup.save()
                    }
                }
            }

            // The site's saved logins.
            ColumnLayout {
                visible: control.prompt === null
                Layout.fillWidth: true
                spacing: Theme.spacing

                Label {
                    Layout.fillWidth: true
                    text: control.logins.length > 0 ? qsTr("Saved for %1").arg(control.host)
                                                    : qsTr("No passwords saved for %1").arg(control.host)
                    color: Theme.text
                    font.pixelSize: Theme.fontSize + 1
                    font.weight: Font.DemiBold
                    elide: Text.ElideMiddle
                }
                Caption {
                    visible: control.logins.length === 0
                    text: Passwords.enabled()
                          ? qsTr("Sign in and Ion offers to save the password. Click a login field for saved ones.")
                          : qsTr("Saving passwords is turned off (passwords.enable).")
                }
                Repeater {
                    model: control.logins

                    delegate: RowLayout {
                        id: loginRow
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: Theme.spacing

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 0
                            Label {
                                Layout.fillWidth: true
                                text: loginRow.modelData.username || qsTr("(no username)")
                                textFormat: Text.PlainText
                                color: Theme.text
                                font.pixelSize: Theme.fontSize
                                elide: Text.ElideRight
                            }
                            Caption {
                                visible: !loginRow.modelData.exact
                                text: loginRow.modelData.host
                                elide: Text.ElideMiddle
                                wrapMode: Text.NoWrap
                            }
                        }
                        IconButton {
                            iconName: "copy"
                            tip: qsTr("Copy username")
                            enabled: loginRow.modelData.username.length > 0
                            onClicked: Passwords.copySecret(loginRow.modelData.username)
                        }
                        IconButton {
                            iconName: "key"
                            tip: qsTr("Copy password")
                            onClicked: {
                                const login = JSON.parse(Passwords.credentials(loginRow.modelData.id))
                                Passwords.copySecret(login.password)
                                clipboardClear.secret = login.password
                                clipboardClear.restart()
                                control.view?.notify(qsTr("Password copied; the clipboard clears in a minute"))
                            }
                        }
                    }
                }
                Rectangle { Layout.fillWidth: true; implicitHeight: Theme.hairline; color: Theme.border }
                DialogButton {
                    Layout.fillWidth: true
                    text: qsTr("Manage passwords…")
                    onClicked: {
                        popup.close()
                        control.manageRequested(control.host)
                    }
                }
            }
        }
    }

    // A copied password leaves the clipboard after a minute, unless
    // something else was copied meanwhile.
    Timer {
        id: clipboardClear
        property string secret: ""
        interval: 60 * 1000
        onTriggered: {
            Passwords.clearCopied(secret)
            secret = ""
        }
    }
}
