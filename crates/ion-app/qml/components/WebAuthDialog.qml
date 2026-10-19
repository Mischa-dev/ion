import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// Ion's dialog for security keys and passkeys (WebAuthn): pick an account,
// enter or set the key's PIN, touch the key, or read why it failed. The
// engine talks to the key; this only shows what it needs from the person.
// Lives inside a BrowserTab and covers only that tab; Esc cancels.
Popup {
    id: dialog

    required property WebEngineView view

    property var request: null
    // The account picked in the SelectAccount step.
    property string account: ""

    readonly property int state: request ? request.state : WebEngineWebAuthUxRequest.WebAuthUxState.NotStarted
    readonly property bool selecting: state === WebEngineWebAuthUxRequest.WebAuthUxState.SelectAccount
    readonly property bool collectingPin: state === WebEngineWebAuthUxRequest.WebAuthUxState.CollectPin
    readonly property bool failed: state === WebEngineWebAuthUxRequest.WebAuthUxState.RequestFailed
    readonly property bool settingPin: collectingPin && request
        && request.pinRequest.reason !== WebEngineWebAuthUxRequest.PinEntryReason.Challenge
    readonly property var lines: {
        if (!request)
            return []
        const pin = request.pinRequest
        const heading = WebAuth.heading(state, request.relyingPartyId, pin.reason)
        if (heading.length === 0)
            return []
        return [heading, WebAuth.detail(state, pin.reason, pin.error, pin.minPinLength,
                                        pin.remainingAttempts, request.requestFailureReason)]
    }

    function show(next) {
        if (request && request !== next)
            request.cancel()
        request = next
        account = next.userNames.length > 0 ? next.userNames[0] : ""
        pinField.text = ""
        newPinField.text = ""
        confirmField.text = ""
        open()
        focusField()
    }

    function focusField() {
        if (collectingPin)
            (settingPin ? newPinField : pinField).forceActiveFocus()
        else
            continueButton.forceActiveFocus()
    }

    readonly property bool canContinue: {
        if (selecting)
            return account.length > 0
        if (!collectingPin)
            return false
        if (settingPin)
            return newPinField.text.length > 0 && newPinField.text === confirmField.text
        return pinField.text.length > 0
    }

    function proceed() {
        if (!request)
            return
        if (failed) {
            request.retry()
            return
        }
        if (!canContinue)
            return
        if (selecting) {
            request.setSelectedAccount(account)
        } else if (collectingPin) {
            request.setPin(settingPin ? newPinField.text : pinField.text)
            pinField.text = ""
            newPinField.text = ""
            confirmField.text = ""
        }
    }

    Connections {
        target: dialog.view
        function onWebAuthUxRequested(request) {
            dialog.show(request)
        }
    }
    Connections {
        target: dialog.request
        function onStateChanged() {
            if (dialog.lines.length === 0) {
                dialog.request = null
                dialog.close()
            } else {
                dialog.focusField()
            }
        }
    }

    // Esc, or the tab going away, cancels the request.
    onClosed: {
        if (request)
            request.cancel()
        request = null
        view.forceActiveFocus()
    }

    parent: view
    anchors.centerIn: parent
    width: Math.min(view.width - Theme.spacing * 4, Theme.tabMaxWidth * 2)
    padding: Theme.spacing * 2.5
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape

    background: Rectangle {
        radius: Theme.radius
        color: Theme.surface
        border.color: Theme.border
        border.width: 1
    }
    Overlay.modal: Rectangle {
        color: Theme.background
        opacity: 0.5
    }

    contentItem: ColumnLayout {
        spacing: Theme.spacing * 2

        Keys.onReturnPressed: dialog.proceed()
        Keys.onEnterPressed: dialog.proceed()

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing * 2

            Text {
                text: "🔑"
                font.pixelSize: Theme.fontSize + 7
                color: Theme.text
            }
            Text {
                Layout.fillWidth: true
                text: dialog.lines.length > 0 ? dialog.lines[0] : ""
                color: Theme.text
                font.pixelSize: Theme.fontSize + 2
                font.bold: true
                wrapMode: Text.Wrap
            }
        }
        Text {
            Layout.fillWidth: true
            text: dialog.lines.length > 1 ? dialog.lines[1] : ""
            color: dialog.failed || (dialog.collectingPin && dialog.request
                    && dialog.request.pinRequest.error !== WebEngineWebAuthUxRequest.PinEntryError.NoError)
                ? Theme.warning : Theme.textMuted
            font.pixelSize: Theme.fontSize
            wrapMode: Text.Wrap
        }

        // Accounts on the key, for SelectAccount.
        Repeater {
            model: dialog.selecting ? dialog.request.userNames : []
            delegate: RadioButton {
                required property string modelData
                Layout.fillWidth: true
                text: modelData
                checked: dialog.account === modelData
                onClicked: dialog.account = modelData
                contentItem: Text {
                    leftPadding: parent.indicator.width + Theme.spacing
                    text: parent.text
                    color: Theme.text
                    font.pixelSize: Theme.fontSize
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }
            }
        }

        // The PIN, to sign in.
        IonTextField {
            id: pinField
            Layout.fillWidth: true
            visible: dialog.collectingPin && !dialog.settingPin
            placeholderText: qsTr("PIN")
            echoMode: TextInput.Password
        }
        IonTextField {
            id: newPinField
            Layout.fillWidth: true
            visible: dialog.settingPin
            placeholderText: qsTr("New PIN")
            echoMode: TextInput.Password
            KeyNavigation.tab: confirmField
        }
        IonTextField {
            id: confirmField
            Layout.fillWidth: true
            visible: dialog.settingPin
            placeholderText: qsTr("Confirm new PIN")
            echoMode: TextInput.Password
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spacing

            DialogButton {
                text: qsTr("Cancel")
                onClicked: dialog.close()
            }
            DialogButton {
                id: continueButton
                visible: dialog.selecting || dialog.collectingPin || dialog.failed
                text: dialog.failed ? qsTr("Try again") : qsTr("Continue")
                primary: true
                enabled: dialog.failed || dialog.canContinue
                onClicked: dialog.proceed()
            }
        }
    }
}
