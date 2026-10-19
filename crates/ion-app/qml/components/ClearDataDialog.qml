import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Ion

// "Clear browsing data": history, cookies and the cache, each optional.
// Open tabs and saved sessions are left alone.
IonDialog {
    id: dialog

    required property var profile

    // Checked when the dialog opens: history and the cache, not cookies
    // (which sign you out everywhere).
    function show() {
        history.checked = true
        cache.checked = true
        cookies.checked = false
        open()
    }

    title: qsTr("Clear browsing data")
    standardButtons: Dialog.Ok | Dialog.Cancel
    Component.onCompleted: standardButton(Dialog.Ok).text = qsTr("Clear")

    onAccepted: {
        if (history.checked) {
            History.clear()
            History.save()
        }
        if (cookies.checked)
            Privacy.clearCookies(dialog.profile)
        if (cache.checked)
            dialog.profile.clearHttpCache()
    }

    component Option: RowLayout {
        property alias checked: toggle.checked
        property string label
        property string help

        Layout.fillWidth: true
        spacing: Theme.spacing * 2

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0
            Text {
                text: label
                color: Theme.text
                font.pixelSize: Theme.fontSize
            }
            Text {
                Layout.fillWidth: true
                text: help
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize - 1
                wrapMode: Text.Wrap
            }
        }
        IonSwitch { id: toggle }
    }

    ColumnLayout {
        width: parent.width
        spacing: Theme.spacing * 2

        Option {
            id: history
            label: qsTr("History")
            help: qsTr("Pages you visited. Open tabs stay.")
        }
        Option {
            id: cookies
            label: qsTr("Cookies and sign-ins")
            help: qsTr("Signs you out of most sites.")
        }
        Option {
            id: cache
            label: qsTr("Cached files")
            help: qsTr("Sites may load slower the next time.")
        }
    }
}
