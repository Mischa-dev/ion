import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Pdf
import QtWebEngine
import Ion

// Print preview (Ctrl+P, a page's window.print(), the PDF viewer's print
// button). The engine prints the tab to a PDF, which is shown page by page;
// "Print" sends the chosen pages to a printer, "Save" keeps the PDF in the
// downloads folder. Changing paper or layout prints the page again.
Popup {
    id: root

    // The tab being printed, while the preview is open.
    property var view: null
    // The engine's PDF of it, once printed.
    property string pdfPath: ""
    property bool rendering: false
    property string error: ""

    readonly property var printers: Printing.printers()
    // Destinations: every printer, then "Save as PDF" (always last).
    readonly property bool saving: destination.currentIndex === destination.count - 1
    readonly property var papers: Printing.papers()
    readonly property string paper: papers[paperChoice.currentIndex]?.id ?? "A4"
    readonly property bool landscape: layoutChoice.currentIndex === 1
    readonly property int pageCount: pdf.status === PdfDocument.Ready ? pdf.pageCount : 0
    readonly property string rangeText: pagesChoice.currentIndex === 1 ? rangeField.text : ""
    readonly property string rangeError: pageCount > 0 ? Printing.rangeError(rangeText, pageCount) : ""
    readonly property var chosenPages: pageCount > 0 ? Printing.pageList(rangeText, pageCount) : []

    function open(tab, saveAsPdf) {
        if (!tab)
            return
        view = tab
        error = ""
        const def = printers.indexOf(Printing.defaultPrinter())
        destination.currentIndex = def >= 0 && !saveAsPdf ? def : destination.count - 1
        pagesChoice.currentIndex = 0
        rangeField.text = ""
        copies.value = 1
        const paperIndex = papers.findIndex(p => p.id === Printing.defaultPaper())
        paperChoice.currentIndex = Math.max(0, paperIndex)
        layoutChoice.currentIndex = 0
        visible = true
        render()
    }

    // Print the tab to a fresh PDF with the current paper and layout.
    function render() {
        if (!view)
            return
        rendering = true
        error = ""
        const target = Printing.previewPath()
        renderTarget = target
        view.printToPdf(target, WebEngineView[paper] ?? WebEngineView.A4,
                        landscape ? WebEngineView.Landscape : WebEngineView.Portrait)
    }
    property string renderTarget: ""

    function accept() {
        if (rendering || pageCount === 0 || rangeError.length > 0 || Printing.busy)
            return
        if (saving) {
            const name = Printing.fileName(view.title, view.url.toString())
            const dir = Downloads.targetDirectory(name, "application/pdf", view.profile.downloadPath)
            const unique = Downloads.uniqueFileName(dir, name)
            if (Printing.copyFile(pdfPath, dir + "/" + unique)) {
                view.notify(qsTr("Saved %1").arg(unique))
                close()
            } else {
                error = qsTr("Couldn't save to %1").arg(dir)
            }
            return
        }
        Printing.print(pdfPath, printers[destination.currentIndex], view.title || view.url.toString(),
                       chosenPages, copies.value, colorChoice.currentIndex === 0, duplex.checked)
    }

    // A tab closed while its preview is open leaves `view` null.
    onViewChanged: if (visible && !view) close()
    // Take the keyboard from the page (the PDF viewer holds on to it), so
    // Esc and Enter reach the preview.
    onOpened: pages.forceActiveFocus()
    onPaperChanged: if (visible) render()
    onLandscapeChanged: if (visible) render()
    onClosed: {
        if (pdfPath.length > 0)
            Printing.removePreview(pdfPath)
        pdfPath = ""
        rendering = false
        const printed = view
        view = null
        printed?.forceActiveFocus()
    }

    Connections {
        target: root.view
        function onPdfPrintingFinished(filePath, success) {
            if (filePath !== root.renderTarget)
                return
            root.rendering = false
            if (!success) {
                root.error = qsTr("This page couldn't be printed.")
                return
            }
            const old = root.pdfPath
            root.pdfPath = filePath
            if (old.length > 0 && old !== filePath)
                Printing.removePreview(old)
        }
    }

    Connections {
        target: Printing
        function onPrinted(message) {
            if (!root.visible)
                return
            if (message.length > 0) {
                root.error = message
            } else {
                root.view?.notify(qsTr("Sent to %1").arg(root.printers[destination.currentIndex]))
                root.close()
            }
        }
    }

    PdfDocument {
        id: pdf
        source: root.pdfPath.length > 0 ? "file://" + root.pdfPath : ""
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(parent ? parent.width - Theme.spacing * 8 : 900, 1100)
    height: parent ? parent.height - Theme.spacing * 8 : 700
    modal: true
    focus: true
    padding: 0
    closePolicy: Popup.CloseOnEscape

    Overlay.modal: Rectangle {
        color: Theme.alpha(Theme.background, Theme.scrimOpacity)
    }

    background: Rectangle {
        radius: Theme.radius
        color: Theme.surface
        border.color: Theme.border
        border.width: Theme.hairline
    }

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
        NumberAnimation { property: "scale"; from: 0.98; to: 1; duration: Theme.animationSlowMs; easing.type: Easing.OutCubic }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; from: 1; to: 0; duration: Theme.animationMs }
    }

    contentItem: RowLayout {
        spacing: 0

        // The pages, as they will come out.
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: Theme.background
            radius: Theme.radius
            clip: true

            ListView {
                id: pages
                anchors.fill: parent
                anchors.margins: Theme.spacing * 3
                spacing: Theme.spacing * 4
                model: root.pageCount
                visible: !root.rendering || root.pageCount > 0
                opacity: root.rendering ? Theme.dimmedOpacity : 1
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: ScrollBar {}

                delegate: Column {
                    id: page
                    required property int index
                    readonly property size points: pdf.pagePointSize(index)
                    readonly property bool chosen: root.chosenPages.indexOf(index) >= 0 || root.saving
                    width: pages.width
                    spacing: Theme.spacing

                    Rectangle {
                        // Fits the view's height, so a whole page shows at once.
                        readonly property real ratio: page.points.width > 0 ? page.points.height / page.points.width : 1.414
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: Math.min(pages.width - Theme.spacing * 4, (pages.height - Theme.spacing * 6) / ratio)
                        height: width * ratio
                        color: Theme.pageCanvas
                        border.width: Theme.hairline
                        border.color: Theme.border
                        opacity: page.chosen ? 1 : Theme.dimmedOpacity

                        PdfPageImage {
                            anchors.fill: parent
                            anchors.margins: Theme.hairline
                            document: pdf
                            currentFrame: page.index
                            sourceSize.width: 1200
                            fillMode: Image.PreserveAspectFit
                            smooth: true
                            mipmap: true
                        }
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: qsTr("%1 of %2").arg(page.index + 1).arg(root.pageCount)
                        color: Theme.textMuted
                        font.pixelSize: Theme.fontSize - 1
                    }
                }
            }

            Text {
                anchors.centerIn: parent
                visible: root.rendering && root.pageCount === 0
                text: qsTr("Preparing preview…")
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize
            }
        }

        // The settings.
        ColumnLayout {
            Layout.fillWidth: false
            Layout.preferredWidth: 300
            Layout.fillHeight: true
            Layout.margins: Theme.spacing * 3
            spacing: Theme.spacing * 2

            Text {
                text: qsTr("Print")
                color: Theme.text
                font.pixelSize: Theme.fontSize + 4
                font.weight: Font.DemiBold
            }
            Text {
                Layout.fillWidth: true
                readonly property int sheets: Math.ceil(root.chosenPages.length / (duplex.checked ? 2 : 1)) * copies.value
                text: root.pageCount === 0 ? " "
                    : root.saving ? (root.pageCount === 1 ? qsTr("1 page") : qsTr("%1 pages").arg(root.pageCount))
                    : sheets === 1 ? qsTr("1 sheet of paper") : qsTr("%1 sheets of paper").arg(sheets)
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize
            }

            component Label: Text {
                Layout.topMargin: Theme.spacing
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize - 1
            }

            Label { text: qsTr("Destination") }
            PrintChoice {
                id: destination
                Layout.fillWidth: true
                model: root.printers.concat([qsTr("Save as PDF")])
            }

            Label { text: qsTr("Pages"); visible: !root.saving }
            PrintChoice {
                id: pagesChoice
                Layout.fillWidth: true
                visible: !root.saving
                model: [qsTr("All"), qsTr("Custom")]
                onActivated: if (currentIndex === 1) rangeField.forceActiveFocus()
            }
            IonTextField {
                id: rangeField
                Layout.fillWidth: true
                visible: !root.saving && pagesChoice.currentIndex === 1
                placeholderText: qsTr("e.g. 1-5, 8, 11-13")
                Keys.onReturnPressed: root.accept()
                Keys.onEnterPressed: root.accept()
            }
            Text {
                Layout.fillWidth: true
                visible: !root.saving && root.rangeError.length > 0 && rangeField.text.length > 0
                text: root.rangeError
                color: Theme.danger
                font.pixelSize: Theme.fontSize - 1
                wrapMode: Text.Wrap
            }

            Label { text: qsTr("Copies"); visible: !root.saving }
            SpinBox {
                id: copies
                visible: !root.saving
                from: 1
                to: 99
                editable: true
                implicitHeight: Theme.urlBarHeight
                palette.base: Theme.surfaceRaised
                palette.text: Theme.text
                palette.button: Theme.surfaceRaised
                palette.buttonText: Theme.text
                palette.mid: Theme.border
                palette.highlight: Theme.accent
            }

            Label { text: qsTr("Layout") }
            PrintChoice {
                id: layoutChoice
                Layout.fillWidth: true
                model: [qsTr("Portrait"), qsTr("Landscape")]
            }

            Label { text: qsTr("Paper") }
            PrintChoice {
                id: paperChoice
                Layout.fillWidth: true
                model: root.papers.map(p => p.label)
            }

            Label { text: qsTr("Color"); visible: !root.saving }
            PrintChoice {
                id: colorChoice
                Layout.fillWidth: true
                visible: !root.saving
                model: [qsTr("Color"), qsTr("Black and white")]
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.topMargin: Theme.spacing
                visible: !root.saving
                Text {
                    Layout.fillWidth: true
                    text: qsTr("Print on both sides")
                    color: Theme.text
                    font.pixelSize: Theme.fontSize
                }
                IonSwitch { id: duplex }
            }

            Item { Layout.fillHeight: true }

            Text {
                Layout.fillWidth: true
                visible: root.error.length > 0
                text: root.error
                color: Theme.danger
                font.pixelSize: Theme.fontSize
                wrapMode: Text.Wrap
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spacing
                Item { Layout.fillWidth: true }
                DialogButton {
                    text: qsTr("Cancel")
                    onClicked: root.close()
                }
                DialogButton {
                    primary: true
                    text: root.saving ? qsTr("Save") : Printing.busy ? qsTr("Printing…") : qsTr("Print")
                    enabled: !root.rendering && root.pageCount > 0 && root.rangeError.length === 0 && !Printing.busy
                    onClicked: root.accept()
                }
            }
        }
    }

    Shortcut {
        sequences: ["Return", "Enter"]
        enabled: root.visible && !rangeField.activeFocus
        onActivated: root.accept()
    }
}
