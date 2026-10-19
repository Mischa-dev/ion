import QtQuick
import QtQuick.Layouts
import Ion

// Offers the tabs from a run that ended without a clean quit (a crash, a
// power cut), when `general.restoreSession` is off so they weren't restored
// already. Sits at the top of the window, over the page, until answered.
Rectangle {
    id: notice

    function offer() { visible = true }

    function answer(restore) {
        visible = false
        if (!restore || !Tabs.restoreCrashedSession())
            Tabs.discardCrashedSession()
    }

    visible: false
    z: 20
    anchors.top: parent.top
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.topMargin: Theme.spacing * 2
    width: Math.min(parent.width - Theme.spacing * 4, row.implicitWidth + Theme.spacing * 4)
    height: row.implicitHeight + Theme.spacing * 3
    radius: Theme.radius
    color: Theme.surface
    border.color: Theme.border
    border.width: 1

    RowLayout {
        id: row
        anchors.fill: parent
        anchors.margins: Theme.spacing * 1.5
        spacing: Theme.spacing * 2

        Text {
            Layout.fillWidth: true
            text: qsTr("Ion didn't close properly. Bring back the tabs you had open?")
            color: Theme.text
            font.pixelSize: Theme.fontSize
            wrapMode: Text.Wrap
        }
        DialogButton {
            text: qsTr("Not now")
            onClicked: notice.answer(false)
        }
        DialogButton {
            text: qsTr("Restore tabs")
            primary: true
            onClicked: notice.answer(true)
        }
    }
}
