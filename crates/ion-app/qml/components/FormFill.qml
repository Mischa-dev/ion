import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebChannel
import QtWebEngine
import Ion

// Saved passwords and addresses on one tab's pages: fills a site's login in
// as its page loads, lists saved logins under a focused login field (and
// saved addresses under an address field), suggests a strong password for
// sign-up forms, and turns a sent form into a save or update prompt
// (`prompt`, shown by the toolbar's PasswordsButton).
//
// The page side is `ion_passwords::PAGE_SCRIPT`, running in an isolated
// world of the top frame, talking over `channel`; the page's own scripts
// can't reach either. Every message is checked against the tab's current
// origin before Ion acts on it.
Item {
    id: root

    property var view   // the BrowserTab

    // The tab's view needs this as its webChannel, in ApplicationWorld.
    readonly property WebChannel channel: WebChannel {
        registeredObjects: [pageBridge]
    }

    // Whether the page has a login form (the key icon shows).
    property bool formOnPage: false
    // A pending offer: {kind: "save"|"update", id, username, password, url},
    // {kind: "address", address, url}, or null.
    property var prompt: null
    // Page loads since the prompt appeared; it lapses after a couple, so it
    // survives a sign-in's redirects but not browsing on.
    property int loadsSincePrompt: 0
    // Emitted when a fresh prompt is ready to show.
    signal promptRaised()
    // Emitted when "Manage…" is picked in the field's list: section
    // "passwords" or "addresses".
    signal manageRequested(string section)

    function dismissPrompt() {
        prompt = null
    }

    anchors.fill: parent

    function install() {
        const lib = WebEngine.script()
        lib.name = "ion-webchannel"
        lib.sourceUrl = "qrc:///qtwebchannel/qwebchannel.js"
        lib.injectionPoint = WebEngineScript.DocumentCreation
        lib.worldId = WebEngineScript.ApplicationWorld
        lib.runsOnSubFrames = false
        view.userScripts.insert(lib)
        const forms = WebEngine.script()
        forms.name = "ion-forms"
        forms.sourceCode = Passwords.pageScript()
        forms.injectionPoint = WebEngineScript.DocumentReady
        forms.worldId = WebEngineScript.ApplicationWorld
        forms.runsOnSubFrames = false
        view.userScripts.insert(forms)
    }
    Component.onCompleted: install()

    // The second line under an address in the list: "1 Main St, Town".
    function addressDetail(a) {
        return [a.street.split("\n")[0], a.city, a.postalCode, a.email]
            .filter(part => part && part.length > 0 && part !== a.name).slice(0, 2).join(", ")
    }

    function current(url) {
        return Passwords.sameOrigin(url, view.url.toString())
    }

    function pageCall(code) {
        view.runJavaScript("window.__ionForms && window.__ionForms." + code, WebEngineScript.ApplicationWorld)
    }

    function fillLogin(id, onlyEmpty) {
        const json = Passwords.credentials(id)
        if (json.length === 0)
            return
        const login = JSON.parse(json)
        pageCall("fill(" + JSON.stringify(login.username) + ", " + JSON.stringify(login.password)
                 + ", " + (onlyEmpty ? "true" : "false") + ")")
        Passwords.markUsed(id)
    }

    function fillAddress(address) {
        pageCall("fillAddress(" + JSON.stringify(address) + ")")
        Passwords.markAddressUsed(address.id)
    }

    // Messages from the page script. The first argument is always the
    // page's address as the page saw it.
    QtObject {
        id: pageBridge
        WebChannel.id: "ionForms"

        function formFound(url) {
            if (!root.current(url))
                return
            root.formOnPage = true
            const id = Passwords.autofillFor(view.url.toString())
            if (id >= 0)
                root.fillLogin(id, true)
        }

        function fieldFocused(url, kind, x, y, width, height) {
            if (!root.current(url) || !view.visible)
                return
            menu.showFor(kind, Qt.rect(x, y, width, height))
        }

        function addressSubmitted(url, json) {
            if (!root.current(url) || !Passwords.addressIsNew(json))
                return
            root.prompt = { kind: "address", address: JSON.parse(json), url: view.url.toString() }
            root.loadsSincePrompt = 0
            root.promptRaised()
        }

        function fieldBlurred(url) {
            if (root.current(url))
                menu.close()
        }

        function menuKey(url, key) {
            if (root.current(url))
                menu.key(key)
        }

        function submitted(url, username, password) {
            if (!root.current(url))
                return
            const offer = JSON.parse(Passwords.propose(view.url.toString(), username, password))
            if (offer.kind.length === 0)
                return
            root.prompt = {
                kind: offer.kind,
                id: offer.id,
                username: offer.kind === "update" ? offer.username : username,
                password: password,
                url: view.url.toString()
            }
            root.loadsSincePrompt = 0
            root.promptRaised()
        }
    }

    Connections {
        target: root.view
        function onLoadingChanged(info) {
            if (info.status === WebEngineView.LoadStartedStatus) {
                root.formOnPage = false
                menu.close()
            } else if (info.status === WebEngineView.LoadSucceededStatus && root.prompt) {
                root.loadsSincePrompt++
                if (root.loadsSincePrompt > 2)
                    root.prompt = null
            }
        }
        function onVisibleChanged() {
            if (!root.view.visible)
                menu.close()
        }
    }

    // The list under a focused login field.
    Popup {
        id: menu

        // Rows: {kind: "login", id, username, host, exact} | {kind:
        // "generate", password} | {kind: "address", address} | {kind:
        // "manage", section}.
        property var rows: []
        property int highlighted: 0
        property rect field

        function showFor(kind, rect) {
            const list = []
            if (kind === "new") {
                list.push({ kind: "generate", password: Passwords.generate() })
            } else if (kind === "address") {
                for (const address of JSON.parse(Passwords.addresses("")))
                    list.push({ kind: "address", address: address })
            } else {
                for (const login of JSON.parse(Passwords.matches(view.url.toString())))
                    list.push(Object.assign({ kind: "login" }, login))
            }
            if (list.length === 0) {
                close()
                return
            }
            list.push({ kind: "manage", section: kind === "address" ? "addresses" : "passwords" })
            rows = list
            highlighted = 0
            field = rect
            open()
        }

        function key(name) {
            if (name === "ArrowDown")
                highlighted = (highlighted + 1) % rows.length
            else if (name === "ArrowUp")
                highlighted = (highlighted + rows.length - 1) % rows.length
            else if (name === "Enter")
                activate(highlighted)
            else
                close()
        }

        function activate(index) {
            const row = rows[index]
            close()
            if (!row)
                return
            if (row.kind === "login")
                root.fillLogin(row.id, false)
            else if (row.kind === "generate")
                root.pageCall("fillNew(" + JSON.stringify(row.password) + ")")
            else if (row.kind === "address")
                root.fillAddress(row.address)
            else
                root.manageRequested(row.section)
        }

        readonly property real zoom: view?.zoomFactor ?? 1
        readonly property real below: (field.y + field.height) * zoom + Theme.spacing / 2
        x: Math.max(0, Math.min(field.x * zoom, root.width - width))
        y: below + height <= root.height ? below : Math.max(0, field.y * zoom - height - Theme.spacing / 2)
        width: Math.max(field.width * zoom, Theme.tabMaxWidth + Theme.spacing * 8)
        padding: Theme.spacing / 2
        // The page keeps focus: typing goes on in the field, and the page
        // script forwards the arrows, Enter and Escape here.
        focus: false
        closePolicy: Popup.CloseOnPressOutside

        onOpened: root.pageCall("menu(true)")
        onClosed: root.pageCall("menu(false)")

        background: Rectangle {
            radius: Theme.radius
            color: Theme.surface
            border.color: Theme.border
            border.width: Theme.hairline
        }

        enter: Transition {
            NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animationMs }
        }

        contentItem: Column {
            spacing: 0

            Repeater {
                model: menu.rows

                delegate: ItemDelegate {
                    id: row

                    required property var modelData
                    required property int index

                    width: menu.availableWidth
                    implicitHeight: modelData.kind === "manage" ? Theme.tabHeight : Theme.tabHeight + Theme.spacing * 2
                    leftPadding: Theme.spacing * 2
                    rightPadding: Theme.spacing * 2
                    focusPolicy: Qt.NoFocus
                    highlighted: index === menu.highlighted
                    onHoveredChanged: if (hovered) menu.highlighted = index
                    onClicked: menu.activate(index)

                    contentItem: RowLayout {
                        spacing: Theme.spacing * 1.5

                        Text {
                            text: row.modelData.kind === "generate" ? "✦" : row.modelData.kind === "manage" ? "⚙"
                                : row.modelData.kind === "address" ? "⌂" : "⚿"
                            color: row.modelData.kind === "generate" ? Theme.accent : Theme.textMuted
                            font.pixelSize: Theme.fontSize + 1
                            Layout.alignment: Qt.AlignVCenter
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 0

                            Text {
                                Layout.fillWidth: true
                                text: row.modelData.kind === "generate" ? qsTr("Use a strong password")
                                    : row.modelData.kind === "manage"
                                      ? (row.modelData.section === "addresses" ? qsTr("Manage addresses…") : qsTr("Manage passwords…"))
                                    : row.modelData.kind === "address" ? (row.modelData.address.name || row.modelData.address.summary)
                                    : row.modelData.username.length > 0 ? row.modelData.username
                                    : qsTr("(no username)")
                                textFormat: Text.PlainText
                                color: Theme.text
                                font.pixelSize: Theme.fontSize
                                elide: Text.ElideRight
                            }
                            Text {
                                Layout.fillWidth: true
                                visible: row.modelData.kind !== "manage"
                                text: row.modelData.kind === "generate" ? row.modelData.password
                                    : row.modelData.kind === "address" ? root.addressDetail(row.modelData.address)
                                    : row.modelData.exact ? "••••••••••" : qsTr("from %1").arg(row.modelData.host)
                                textFormat: Text.PlainText
                                color: Theme.textMuted
                                font.pixelSize: Theme.fontSize - 1
                                font.family: row.modelData.kind === "generate" ? "monospace" : undefined
                                elide: Text.ElideRight
                            }
                        }
                    }

                    background: Rectangle {
                        radius: Math.max(0, Theme.radius - 2)
                        color: row.highlighted ? Theme.surfaceHover : "transparent"
                    }
                }
            }
        }
    }
}
