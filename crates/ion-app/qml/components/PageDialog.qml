import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// Ion's own alert(), confirm(), prompt(), "leave this page?" and HTTP sign-in
// dialogs, in place of Qt's unthemed defaults. Lives inside a BrowserTab and
// covers only that tab. The heading names the page's real origin, and after a
// page has opened a couple of dialogs it can be stopped from opening more
// until it navigates.
Popup {
    id: dialog

    required property WebEngineView view

    // The request being answered: a JavaScriptDialogRequest when `isAuth` is
    // false, an AuthenticationDialogRequest otherwise.
    property var request: null
    property bool isAuth: false
    property bool answered: false
    // Dialogs since the page last loaded, and whether the person blocked more.
    property int pageDialogCount: 0
    property bool pageBlocked: false

    readonly property int dialogType: !isAuth && request ? request.type : 0
    readonly property bool isPrompt: dialogType === JavaScriptDialogRequest.DialogTypePrompt
    readonly property string rejectLabel: isAuth ? qsTr("Cancel") : Basics.dialogRejectLabel(dialogType)

    function showJavaScript(next) {
        next.accepted = true
        if (pageBlocked) {
            next.dialogReject()
            return
        }
        pageDialogCount++
        // Clear first so no binding sees the new request with the old kind.
        request = null
        isAuth = false
        request = next
        promptField.text = next.defaultText
        blockBox.checked = false
        present()
    }

    function showAuth(next) {
        next.accepted = true
        request = null
        isAuth = true
        request = next
        userField.text = ""
        passwordField.text = ""
        present()
    }

    function present() {
        answered = false
        open()
        if (isAuth)
            userField.forceActiveFocus()
        else if (isPrompt)
            promptField.forceActiveFocus()
        else
            acceptButton.forceActiveFocus()
    }

    function accept() {
        if (!request)
            return
        if (isAuth)
            request.dialogAccept(userField.text, passwordField.text)
        else if (isPrompt)
            request.dialogAccept(promptField.text)
        else
            request.dialogAccept()
        finish()
    }

    function reject() {
        if (!request)
            return
        request.dialogReject()
        finish()
    }

    function finish() {
        if (!isAuth && blockBox.visible && blockBox.checked)
            pageBlocked = true
        answered = true
        close()
    }

    Connections {
        target: dialog.view
        function onJavaScriptDialogRequested(request) {
            dialog.showJavaScript(request)
        }
        function onAuthenticationDialogRequested(request) {
            dialog.showAuth(request)
        }
        function onLoadingChanged(info) {
            if (info.status === WebEngineView.LoadStartedStatus) {
                dialog.pageDialogCount = 0
                dialog.pageBlocked = false
            }
        }
    }

    // Esc, or the tab going away, counts as Cancel.
    onClosed: {
        if (request && !answered)
            request.dialogReject()
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

        Keys.onReturnPressed: dialog.accept()
        Keys.onEnterPressed: dialog.accept()

        Text {
            Layout.fillWidth: true
            text: !dialog.request ? ""
                : dialog.isAuth ? Basics.authHeading(dialog.request.type === AuthenticationDialogRequest.AuthenticationTypeProxy,
                                                     dialog.request.url, dialog.request.proxyHost)
                : Basics.dialogHeading(dialog.dialogType, dialog.request.securityOrigin)
            color: Theme.text
            font.pixelSize: Theme.fontSize + 2
            font.bold: true
            elide: Text.ElideRight
        }

        // The page's message; long ones scroll.
        ScrollView {
            Layout.fillWidth: true
            Layout.preferredHeight: Math.min(message.implicitHeight, dialog.view.height / 2)
            visible: message.text.length > 0
            clip: true

            Text {
                id: message
                width: dialog.availableWidth
                text: !dialog.request ? ""
                    : dialog.isAuth ? Basics.authDetail(dialog.request.realm, dialog.request.url,
                                                        dialog.request.type === AuthenticationDialogRequest.AuthenticationTypeProxy)
                    : dialog.dialogType === JavaScriptDialogRequest.DialogTypeBeforeUnload ? Basics.beforeUnloadMessage()
                    : dialog.request.message
                color: Theme.text
                font.pixelSize: Theme.fontSize
                wrapMode: Text.Wrap
                textFormat: Text.PlainText
            }
        }

        DialogField {
            id: promptField
            Layout.fillWidth: true
            visible: dialog.isPrompt
        }

        DialogField {
            id: userField
            Layout.fillWidth: true
            visible: dialog.isAuth
            placeholderText: qsTr("Username")
            KeyNavigation.tab: passwordField
        }
        DialogField {
            id: passwordField
            Layout.fillWidth: true
            visible: dialog.isAuth
            placeholderText: qsTr("Password")
            echoMode: TextInput.Password
        }

        CheckBox {
            id: blockBox
            visible: !dialog.isAuth && dialog.dialogType !== JavaScriptDialogRequest.DialogTypeBeforeUnload
                     && Basics.offerDialogBlock(dialog.pageDialogCount)
            focusPolicy: Qt.NoFocus
            padding: 0
            text: qsTr("Don't let this page open more dialogs")

            indicator: Rectangle {
                implicitWidth: Theme.fontSize + 4
                implicitHeight: implicitWidth
                y: (blockBox.height - height) / 2
                radius: Theme.radius / 2
                color: blockBox.checked ? Theme.accent : Theme.surfaceRaised
                border.color: Theme.border
                border.width: 1

                Text {
                    anchors.centerIn: parent
                    visible: blockBox.checked
                    text: "✓"
                    color: Theme.background
                    font.pixelSize: Theme.fontSize - 1
                    font.bold: true
                }
            }
            contentItem: Text {
                leftPadding: blockBox.indicator.width + Theme.spacing
                text: blockBox.text
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize - 1
                verticalAlignment: Text.AlignVCenter
                wrapMode: Text.Wrap
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Theme.spacing

            DialogButton {
                visible: dialog.rejectLabel.length > 0
                text: dialog.rejectLabel
                onClicked: dialog.reject()
            }
            DialogButton {
                id: acceptButton
                primary: true
                text: dialog.isAuth ? qsTr("Sign in") : Basics.dialogAcceptLabel(dialog.dialogType)
                onClicked: dialog.accept()
            }
        }
    }

    component DialogField: TextField {
        id: field
        implicitHeight: Theme.urlBarHeight
        color: Theme.text
        placeholderTextColor: Theme.textMuted
        selectionColor: Theme.accent
        selectedTextColor: Theme.background
        font.pixelSize: Theme.fontSize
        leftPadding: Theme.spacing * 2
        rightPadding: Theme.spacing * 2
        verticalAlignment: TextInput.AlignVCenter
        selectByMouse: true

        background: Rectangle {
            radius: Theme.radius
            color: Theme.surfaceRaised
            border.width: field.activeFocus ? 1 : 0
            border.color: Theme.accent
        }
    }
}
