import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import Ion

// The password manager: every saved login, searchable, with a checkup for
// weak and reused passwords; reveal, copy, edit and delete; import from a
// CSV export of another browser or manager, and export. Its second page
// holds saved addresses (AddressBook).
Popup {
    id: root

    property string query: ""
    // "passwords" or "addresses".
    property string section: "passwords"
    readonly property bool onAddresses: section === "addresses"
    // "all", "weak" or "reused".
    property string filter: "all"
    property int selected: -1
    property bool editing: false
    property bool reveal: false
    property bool confirmDelete: false
    property string message: ""

    readonly property string problem: {
        Passwords.revision
        return root.opened ? Passwords.problem() : ""
    }
    readonly property var logins: {
        Passwords.revision
        if (!root.opened)
            return []
        const all = JSON.parse(Passwords.list(root.query))
        return root.filter === "weak" ? all.filter(l => l.weak)
             : root.filter === "reused" ? all.filter(l => l.reused) : all
    }
    readonly property var checkup: {
        Passwords.revision
        return root.opened ? JSON.parse(Passwords.checkup()) : ({})
    }
    readonly property var never: {
        Passwords.revision
        return root.opened ? JSON.parse(Passwords.neverSaved()) : []
    }
    readonly property var current: logins.find(l => l.id === selected) ?? null

    function show(text, page) {
        section = page || "passwords"
        addressBook.reset()
        query = text
        search.text = text
        filter = "all"
        message = ""
        select(-1)
        open()
    }

    function select(id) {
        selected = id
        editing = false
        reveal = false
        confirmDelete = false
        refresh()
    }

    // Typing into the fields breaks bindings, so they are filled by hand
    // whenever the selection or the mode changes.
    function refresh() {
        const login = current
        usernameField.text = login?.username ?? ""
        noteField.text = login?.note ?? ""
        passwordField.text = !login ? "" : reveal || editing ? passwordOf(login.id) : "••••••••••••"
    }
    onRevealChanged: refresh()
    onEditingChanged: refresh()

    function passwordOf(id) {
        const json = Passwords.credentials(id)
        return json.length > 0 ? JSON.parse(json).password : ""
    }

    function copy(text, what) {
        Passwords.copySecret(text)
        clipboardClear.secret = text
        clipboardClear.restart()
        message = what
    }

    // "3 days ago" for a millisecond timestamp, "never" for 0.
    function ago(ms) {
        if (!ms)
            return qsTr("never")
        const days = Math.floor((Date.now() - ms) / 86400000)
        if (days < 1)
            return qsTr("today")
        if (days === 1)
            return qsTr("yesterday")
        if (days < 60)
            return qsTr("%1 days ago").arg(days)
        return new Date(ms).toLocaleDateString(Qt.locale(), Locale.ShortFormat)
    }

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Theme.urlBarHeight * 2
    width: Math.min(Theme.paletteWidth * 1.4, parent.width - Theme.spacing * 4)
    height: Math.min(parent.height - y - Theme.urlBarHeight * 2, Theme.paletteWidth)
    padding: Theme.spacing * 3
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    onOpened: Qt.callLater(() => search.forceActiveFocus())

    Overlay.modal: Rectangle {
        color: Qt.rgba(Theme.background.r, Theme.background.g, Theme.background.b, Theme.scrimOpacity)
    }
    background: Rectangle {
        radius: Theme.radius
        color: Theme.surface
        border.width: Theme.hairline
        border.color: Theme.border
    }
    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
        NumberAnimation { property: "scale"; from: 0.97; to: 1; duration: Theme.animationSlowMs; easing.type: Easing.OutCubic }
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

    component Chip: AbstractButton {
        id: chip
        property bool selectedChip: false
        property color tint: Theme.accent
        implicitHeight: Theme.tabHeight - Theme.spacing
        leftPadding: Theme.spacing * 1.5
        rightPadding: Theme.spacing * 1.5
        focusPolicy: Qt.NoFocus
        contentItem: Text {
            text: chip.text
            color: chip.selectedChip ? Theme.onAccent : Theme.text
            font.pixelSize: Theme.fontSize - 1
            verticalAlignment: Text.AlignVCenter
        }
        background: Rectangle {
            radius: height / 2
            color: chip.selectedChip ? chip.tint : chip.hovered ? Theme.surfaceHover : Theme.surfaceRaised
        }
    }

    contentItem: ColumnLayout {
        spacing: Theme.spacing * 1.5

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing

            // The two pages, as headings to switch between.
            Repeater {
                model: [
                    { page: "passwords", title: qsTr("Passwords"), count: root.checkup.total },
                    { page: "addresses", title: qsTr("Addresses"), count: addressBook.addresses.length }
                ]
                delegate: AbstractButton {
                    id: pageButton
                    required property var modelData
                    readonly property bool here: root.section === modelData.page
                    focusPolicy: Qt.NoFocus
                    rightPadding: Theme.spacing * 2
                    onClicked: {
                        root.section = modelData.page
                        root.message = ""
                        search.text = ""
                    }
                    contentItem: Row {
                        spacing: Theme.spacing
                        Label {
                            id: pageTitle
                            text: pageButton.modelData.title
                            color: pageButton.here ? Theme.text : Theme.textMuted
                            font.pixelSize: Theme.fontSize + 4
                            font.weight: Font.DemiBold
                        }
                        Label {
                            anchors.baseline: pageTitle.baseline
                            text: pageButton.modelData.count !== undefined && pageButton.modelData.count > 0
                                  ? pageButton.modelData.count : ""
                            color: Theme.textMuted
                            font.pixelSize: Theme.fontSize + 1
                        }
                    }
                    HoverHandler { cursorShape: Qt.PointingHandCursor }
                }
            }
            Item { Layout.fillWidth: true }
            DialogButton { visible: !root.onAddresses; text: qsTr("Import…"); onClicked: importPicker.open() }
            DialogButton {
                visible: !root.onAddresses
                text: qsTr("Export…")
                enabled: (root.checkup.total ?? 0) > 0
                onClicked: exportConfirm.open()
            }
            IconButton { glyph: "✕"; tip: qsTr("Close"); onClicked: root.close() }
        }

        // Where the secrets are, or why they can't be used.
        Caption {
            text: root.problem.length > 0 ? root.problem
                : Passwords.storeIsKeyring()
                  ? qsTr("Encrypted on this device. The key is in your %1.").arg(Passwords.storeName())
                  : qsTr("Encrypted on this device. No system keyring answered, so the key is in a private file in Ion's data folder.")
            color: root.problem.length > 0 ? Theme.danger : Theme.textMuted
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing

            IonTextField {
                id: search
                Layout.fillWidth: true
                placeholderText: root.onAddresses ? qsTr("Search addresses") : qsTr("Search sites and usernames")
                onTextChanged: {
                    if (root.onAddresses)
                        addressBook.query = text
                    else
                        root.query = text
                }
                Keys.onDownPressed: list.forceActiveFocus()
            }
            Chip {
                visible: !root.onAddresses
                text: qsTr("All")
                selectedChip: root.filter === "all"
                onClicked: root.filter = "all"
            }
            Chip {
                text: qsTr("%n weak", "", root.checkup.weak ?? 0)
                visible: !root.onAddresses && (root.checkup.weak ?? 0) > 0
                tint: Theme.warning
                selectedChip: root.filter === "weak"
                onClicked: root.filter = "weak"
            }
            Chip {
                text: qsTr("%n reused", "", root.checkup.reused ?? 0)
                visible: !root.onAddresses && (root.checkup.reused ?? 0) > 0
                tint: Theme.warning
                selectedChip: root.filter === "reused"
                onClicked: root.filter = "reused"
            }
        }

        AddressBook {
            id: addressBook
            visible: root.onAddresses
            active: root.opened
            Layout.fillWidth: true
            Layout.fillHeight: true
        }

        RowLayout {
            visible: !root.onAddresses
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.spacing * 2

            // Logins.
            ListView {
                id: list
                Layout.preferredWidth: Theme.tabMaxWidth + Theme.spacing * 10
                Layout.fillHeight: true
                clip: true
                model: root.logins
                currentIndex: root.logins.findIndex(l => l.id === root.selected)
                keyNavigationEnabled: true
                ScrollBar.vertical: ScrollBar {}
                Keys.onUpPressed: { if (currentIndex > 0) root.select(root.logins[currentIndex - 1].id) }
                Keys.onDownPressed: {
                    if (currentIndex + 1 < count)
                        root.select(root.logins[currentIndex + 1].id)
                }

                delegate: ItemDelegate {
                    id: row
                    required property var modelData
                    width: ListView.view.width
                    implicitHeight: Theme.tabHeight + Theme.spacing * 2.5
                    leftPadding: Theme.spacing * 1.5
                    rightPadding: Theme.spacing * 1.5
                    focusPolicy: Qt.NoFocus
                    onClicked: root.select(modelData.id)

                    contentItem: RowLayout {
                        spacing: Theme.spacing
                        // Initial of the site, as a small tile.
                        Rectangle {
                            implicitWidth: Theme.iconSize + Theme.spacing
                            implicitHeight: implicitWidth
                            radius: Theme.radius / 2
                            color: Theme.surfaceRaised
                            Text {
                                anchors.centerIn: parent
                                text: row.modelData.host.charAt(0).toUpperCase()
                                color: Theme.text
                                font.pixelSize: Theme.fontSize - 1
                                font.weight: Font.DemiBold
                            }
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 0
                            Label {
                                Layout.fillWidth: true
                                text: row.modelData.host
                                color: Theme.text
                                font.pixelSize: Theme.fontSize
                                elide: Text.ElideMiddle
                            }
                            Label {
                                Layout.fillWidth: true
                                text: row.modelData.username || qsTr("(no username)")
                                textFormat: Text.PlainText
                                color: Theme.textMuted
                                font.pixelSize: Theme.fontSize - 1
                                elide: Text.ElideRight
                            }
                        }
                        Rectangle {
                            visible: row.modelData.weak || row.modelData.reused
                            implicitWidth: Theme.spacing + 1
                            implicitHeight: implicitWidth
                            radius: width / 2
                            color: Theme.warning
                        }
                    }
                    background: Rectangle {
                        radius: Theme.radius
                        color: row.modelData.id === root.selected ? Theme.surfaceRaised
                             : row.hovered ? Theme.surfaceHover : "transparent"
                    }
                }

                Label {
                    anchors.centerIn: parent
                    width: parent.width - Theme.spacing * 4
                    visible: list.count === 0 && root.problem.length === 0
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.Wrap
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSize
                    text: root.query.length > 0 || root.filter !== "all" ? qsTr("Nothing matches.")
                        : qsTr("No saved passwords yet. Ion offers to save them as you sign in, or import them from another browser.")
                }
            }

            Rectangle { Layout.fillHeight: true; implicitWidth: Theme.hairline; color: Theme.border }

            // The selected login.
            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: Theme.spacing * 1.5

                Caption {
                    visible: root.current === null
                    Layout.fillHeight: true
                    verticalAlignment: Text.AlignVCenter
                    horizontalAlignment: Text.AlignHCenter
                    text: root.never.length > 0
                          ? qsTr("Pick a login to see it.\n\nNever saved on: %1").arg(root.never.map(n => n.host).join(", "))
                          : qsTr("Pick a login to see it.")
                }

                ColumnLayout {
                    visible: root.current !== null
                    Layout.fillWidth: true
                    spacing: Theme.spacing * 1.5

                    Label {
                        Layout.fillWidth: true
                        text: root.current?.host ?? ""
                        color: Theme.text
                        font.pixelSize: Theme.fontSize + 3
                        font.weight: Font.DemiBold
                        elide: Text.ElideMiddle
                    }
                    Label {
                        Layout.fillWidth: true
                        text: root.current?.origin ?? ""
                        color: Theme.accent
                        font.pixelSize: Theme.fontSize - 1
                        elide: Text.ElideMiddle
                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: Qt.openUrlExternally(root.current.origin)
                        }
                    }

                    Caption { text: qsTr("Username") }
                    RowLayout {
                        Layout.fillWidth: true
                        IonTextField {
                            id: usernameField
                            Layout.fillWidth: true
                            readOnly: !root.editing
                        }
                        IconButton {
                            glyph: "⧉"
                            tip: qsTr("Copy username")
                            onClicked: root.copy(usernameField.text, qsTr("Username copied"))
                        }
                    }

                    Caption { text: qsTr("Password") }
                    RowLayout {
                        Layout.fillWidth: true
                        IonTextField {
                            id: passwordField
                            Layout.fillWidth: true
                            readOnly: !root.editing
                            echoMode: root.reveal || root.editing ? TextInput.Normal : TextInput.Password
                            font.family: root.reveal || root.editing ? "monospace" : undefined
                        }
                        IconButton {
                            visible: root.editing
                            glyph: "✦"
                            tip: qsTr("Generate a strong password")
                            onClicked: passwordField.text = Passwords.generate()
                        }
                        IconButton {
                            visible: !root.editing
                            glyph: root.reveal ? "◉" : "◎"
                            tip: root.reveal ? qsTr("Hide password") : qsTr("Show password")
                            onClicked: root.reveal = !root.reveal
                        }
                        IconButton {
                            glyph: "⚿"
                            tip: qsTr("Copy password")
                            onClicked: root.copy(root.editing ? passwordField.text : root.passwordOf(root.selected),
                                                 qsTr("Password copied; the clipboard clears in a minute"))
                        }
                    }
                    Caption {
                        visible: (root.current?.weak ?? false) || (root.current?.reused ?? false)
                        color: Theme.warning
                        text: root.current?.reused
                              ? qsTr("This password is used on other sites too. If one of them leaks, this account is at risk: change it.")
                              : qsTr("This password is easy to guess. Change it on the site, then here.")
                    }

                    Caption { text: qsTr("Note"); visible: root.editing || (root.current?.note ?? "").length > 0 }
                    IonTextField {
                        id: noteField
                        Layout.fillWidth: true
                        visible: root.editing || (root.current?.note ?? "").length > 0
                        readOnly: !root.editing
                    }

                    Caption {
                        text: root.current
                              ? qsTr("Saved %1 · changed %2 · last used %3").arg(root.ago(root.current.created))
                                .arg(root.ago(root.current.changed)).arg(root.ago(root.current.used))
                              : ""
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.spacing

                        DialogButton {
                            visible: !root.editing
                            text: root.confirmDelete ? qsTr("Really delete?") : qsTr("Delete")
                            onClicked: {
                                if (!root.confirmDelete) {
                                    root.confirmDelete = true
                                    return
                                }
                                Passwords.remove(root.selected)
                                root.select(-1)
                            }
                        }
                        Item { Layout.fillWidth: true }
                        DialogButton {
                            visible: root.editing
                            text: qsTr("Cancel")
                            onClicked: {
                                const id = root.selected
                                root.select(id)
                            }
                        }
                        DialogButton {
                            primary: root.editing
                            text: root.editing ? qsTr("Save") : qsTr("Edit")
                            enabled: !root.editing || passwordField.text.length > 0
                            onClicked: {
                                if (!root.editing) {
                                    root.editing = true
                                    return
                                }
                                Passwords.edit(root.selected, usernameField.text, passwordField.text, noteField.text)
                                root.editing = false
                                root.message = qsTr("Saved")
                            }
                        }
                    }
                }

                Item { Layout.fillHeight: true; visible: root.current !== null }
                Caption {
                    visible: root.message.length > 0
                    text: root.message
                    color: Theme.text
                }
            }
        }
    }

    FileDialog {
        id: importPicker
        title: qsTr("Import passwords from a CSV file")
        nameFilters: [qsTr("Password exports (*.csv)"), qsTr("All files (*)")]
        onAccepted: {
            const report = JSON.parse(Passwords.importFile(selectedFile.toString()))
            root.message = report.error !== undefined
                ? qsTr("Couldn't import: %1").arg(report.error)
                : qsTr("Imported %1 new and %2 updated passwords (%3 already saved or not for a website). Delete the CSV file now: it holds every password in plain text.")
                  .arg(report.added).arg(report.updated).arg(report.skipped)
        }
    }

    IonDialog {
        id: exportConfirm
        title: qsTr("Export passwords?")
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: exportPicker.open()
        Label {
            width: parent.width
            wrapMode: Text.Wrap
            color: Theme.text
            font.pixelSize: Theme.fontSize
            text: qsTr("The file holds every saved password in plain text, readable by anyone who gets it. Import it where you need it, then delete it.")
        }
    }

    FileDialog {
        id: exportPicker
        title: qsTr("Export passwords")
        fileMode: FileDialog.SaveFile
        defaultSuffix: "csv"
        nameFilters: [qsTr("CSV files (*.csv)")]
        onAccepted: {
            const error = Passwords.exportFile(selectedFile.toString())
            root.message = error.length > 0 ? qsTr("Couldn't export: %1").arg(error)
                                            : qsTr("Exported. Delete the file once you've imported it elsewhere.")
        }
    }

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
