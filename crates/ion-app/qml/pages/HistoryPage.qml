import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

// ion://history: pages visited, newest first, grouped by day, with search.
// Arrow keys move through the list, Enter opens, Delete forgets a page.
LibraryFrame {
    id: page


    // How many pages to list; grows as the list is scrolled to its end.
    property int limit: 300
    property int total: 0

    function dayLabel(seconds) {
        const date = new Date(seconds * 1000)
        const today = new Date()
        today.setHours(0, 0, 0, 0)
        const day = new Date(date)
        day.setHours(0, 0, 0, 0)
        const days = Math.round((today - day) / 86400000)
        if (days === 0)
            return qsTr("Today")
        if (days === 1)
            return qsTr("Yesterday")
        if (days < 7)
            return Qt.locale().dayName(date.getDay(), Locale.LongFormat)
        return date.toLocaleDateString(Qt.locale(), date.getFullYear() === today.getFullYear()
                                       ? "d MMMM" : "d MMMM yyyy")
    }

    function refresh() {
        const entries = JSON.parse(History.recent(query, limit))
        total = entries.length
        entries.forEach(e => {
            e.day = dayLabel(e.lastVisit)
            e.time = new Date(e.lastVisit * 1000).toLocaleTimeString(Qt.locale(), Locale.ShortFormat)
        })
        // Keep the list's place when only something below it changed.
        const keep = list.contentY
        model.clear()
        model.append(entries)
        list.contentY = keep
        if (list.currentIndex >= model.count)
            list.currentIndex = model.count - 1
    }

    function openEntry(index, newTab) {
        const entry = model.get(index)
        if (!entry)
            return
        if (newTab)
            browser.openTab(entry.url, false)
        else
            view.url = entry.url
    }

    function removeEntry(index) {
        const entry = model.get(index)
        if (entry)
            History.remove(entry.url)
    }

    current: "history"
    title: qsTr("History")
    placeholder: qsTr("Search history")
    onQueryChanged: {
        limit = 300
        refreshTimer.restart()
    }
    Component.onCompleted: refresh()

    // Many visits can land at once (a page with redirects); refresh once.
    Timer {
        id: refreshTimer
        interval: 150
        onTriggered: page.refresh()
    }
    Connections {
        target: History
        function onChanged() { refreshTimer.restart() }
    }

    actions: [
        DialogButton {
            text: qsTr("Clear browsing data…")
            onClicked: page.browser.showClearData()
        }
    ]

    ListModel { id: model }

    ListView {
        id: list
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spacing * 4
        anchors.horizontalCenter: parent.horizontalCenter
        width: Math.min(parent.width - Theme.spacing * 8, Theme.readableWidth + Theme.tabMaxWidth)
        clip: true
        model: model
        focus: true
        keyNavigationEnabled: true
        currentIndex: -1
        highlightMoveDuration: 0
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {}

        onAtYEndChanged: if (atYEnd && page.total >= page.limit) {
            page.limit += 300
            page.refresh()
        }

        Keys.onReturnPressed: event => page.openEntry(currentIndex, event.modifiers & Qt.ControlModifier)
        Keys.onDeletePressed: page.removeEntry(currentIndex)

        section.property: "day"
        section.delegate: Text {
            required property string section
            width: ListView.view.width
            topPadding: Theme.spacing * 4
            bottomPadding: Theme.spacing
            leftPadding: Theme.spacing * 2
            text: section
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize - 1
            font.weight: Font.DemiBold
        }

        delegate: LibraryRow {
            id: entry
            required property int index
            required property var model

            width: ListView.view.width
            url: model.url
            title: model.title
            note: model.time
            selected: ListView.isCurrentItem && list.activeFocus
            onOpen: newTab => page.openEntry(index, newTab)
            onRemoveRequested: page.removeEntry(index)
            onMenuRequested: {
                list.currentIndex = index
                menu.entry = { url: entry.url, host: entry.host }
                menu.popup()
            }
            removeTip: qsTr("Remove from history")
        }
    }

    Column {
        anchors.centerIn: parent
        visible: model.count === 0
        spacing: Theme.spacing
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: page.query.length > 0 ? qsTr("Nothing in history matches “%1”").arg(page.query)
                                        : qsTr("Pages you visit show up here")
            color: Theme.textMuted
            font.pixelSize: Theme.fontSize + 2
        }
    }

    IonMenu {
        id: menu
        property var entry: ({})

        IonMenuItem {
            text: qsTr("Open in new tab")
            onTriggered: page.browser.openTab(menu.entry.url, false)
        }
        IonMenuItem {
            text: qsTr("Copy link")
            onTriggered: {
                clipboard.text = menu.entry.url
                clipboard.selectAll()
                clipboard.copy()
            }
        }
        IonMenuSeparator {}
        IonMenuItem {
            text: qsTr("Remove from history")
            onTriggered: History.remove(menu.entry.url)
        }
        IonMenuItem {
            text: qsTr("Forget everything from %1").arg(menu.entry.host || "")
            onTriggered: History.removeSite(menu.entry.host)
        }
    }

    TextEdit { id: clipboard; visible: false }
}
