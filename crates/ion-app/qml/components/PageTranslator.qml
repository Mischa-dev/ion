import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// Page translation for one tab, into your system's language, through the
// model Ion Agent uses (Translator). Nothing leaves the browser until you
// ask: the "Translate page" command starts it. While it runs, and after,
// a pill at the top of the page says so, with the way back to the original.
Rectangle {
    id: bar

    required property var view

    // "", "translating" or "translated".
    property string state: ""
    property int job: 0
    // Batches translated so far in this job.
    property int received: 0
    // Where pages are translated to: the system's language, English when
    // the system names none (the "C" locale).
    readonly property bool noLocale: Qt.locale().name === "C" || Qt.locale().name.length === 0
    readonly property string target: noLocale ? "en-US" : Qt.locale().name.replace("_", "-")
    readonly property string targetName: {
        const name = noLocale ? "English" : Qt.locale().nativeLanguageName
        return name.length > 0 ? name.charAt(0).toUpperCase() + name.slice(1) : target
    }

    function translate() {
        const reason = Translator.unavailableReason()
        if (reason.length > 0) {
            view.notify(reason)
            return
        }
        cancel()
        view.runJavaScript(Translator.collectScript, WebEngineScript.ApplicationWorld, page => {
            if (!page) {
                bar.view.notify(qsTr("Ion couldn't read this page's text."))
                return
            }
            if (Translator.alreadyIn(page, bar.target))
                bar.view.notify(qsTr("This page says it's already in %1; translating anyway.").arg(bar.targetName))
            bar.job = Math.floor(Math.random() * 2000000000) + 1
            bar.received = 0
            bar.state = "translating"
            Translator.start(bar.job, page, bar.target)
        })
    }

    function cancel() {
        if (job !== 0)
            Translator.cancel(job)
        job = 0
    }

    function showOriginal() {
        cancel()
        view.runJavaScript(Translator.restoreScript, WebEngineScript.ApplicationWorld)
        state = ""
    }

    Connections {
        target: Translator
        function onBatchDone(job, script) {
            if (job !== bar.job)
                return
            bar.view.runJavaScript(script, WebEngineScript.ApplicationWorld)
            bar.received++
        }
        function onFinished(job, error) {
            if (job !== bar.job)
                return
            bar.job = 0
            if (error.length > 0)
                bar.view.notify(error)
            // Nothing came back at all: there's no original to go back to.
            bar.state = bar.received > 0 ? "translated" : ""
        }
    }

    // A new page is untranslated.
    Connections {
        target: bar.view
        function onUrlChanged() {
            bar.cancel()
            bar.state = ""
        }
    }

    visible: state.length > 0
    z: 10
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: parent.top
    anchors.topMargin: Theme.spacing * 2
    width: row.implicitWidth + Theme.spacing * 3
    height: Theme.urlBarHeight + Theme.spacing
    radius: height / 2
    color: Theme.surface
    border.color: Theme.border
    border.width: Theme.hairline

    RowLayout {
        id: row
        anchors.centerIn: parent
        spacing: Theme.spacing

        Icon {
            name: "globe"
            color: bar.state === "translating" ? Theme.accent : Theme.textMuted
            Layout.leftMargin: Theme.spacing
        }
        Label {
            text: bar.state === "translating"
                ? qsTr("Translating to %1…").arg(bar.targetName)
                : qsTr("Translated to %1").arg(bar.targetName)
            color: Theme.text
            font.pixelSize: Theme.fontSize
        }
        DialogButton {
            text: bar.state === "translating" ? qsTr("Stop") : qsTr("Show original")
            onClicked: bar.showOriginal()
        }
    }
}
