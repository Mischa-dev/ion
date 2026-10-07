import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Ion

ApplicationWindow {
    id: window

    readonly property string homeUrl: Config.homePage
    readonly property var currentView: views.count > 0 ? views.itemAt(tabs.currentIndex) : null

    width: 1280
    height: 820
    visible: true
    color: Theme.background
    title: currentView && currentView.title ? currentView.title + " — Ion" : "Ion"

    // Persistent profile: cookies, cache and storage survive restarts.
    WebEngineProfilePrototype {
        id: profilePrototype
        storageName: "Default"
    }
    // Set in Component.onCompleted: instance() is null until the prototype is complete.
    property WebEngineProfile profile: null

    // Tab state lives here; TabStrip and the view stack both read it.
    ListModel {
        id: tabs
        property int currentIndex: 0
    }

    function openTab(target, activate) {
        tabs.append({ title: "", initialUrl: target ? target.toString() : "" })
        if (activate !== false)
            tabs.currentIndex = tabs.count - 1
        return views.itemAt(tabs.count - 1)
    }

    function closeTab(index) {
        if (index < 0 || index >= tabs.count)
            return
        if (tabs.count === 1) {
            Qt.quit()
            return
        }
        if (index < tabs.currentIndex || (index === tabs.currentIndex && index === tabs.count - 1))
            tabs.currentIndex -= 1
        tabs.remove(index)
    }

    function cycleTab(step) {
        tabs.currentIndex = (tabs.currentIndex + step + tabs.count) % tabs.count
    }

    function focusUrlBar() {
        navBar.urlBar.forceActiveFocus()
        navBar.urlBar.selectAll()
    }

    Component.onCompleted: {
        profile = profilePrototype.instance()
        Adblock.attach(window.profile)
        // URLs on the command line (`ion %U` from the desktop file) open as tabs.
        const urls = Qt.application.arguments.slice(1).filter(arg => !arg.startsWith("-"))
        if (urls.length === 0)
            openTab(homeUrl)
        for (const arg of urls)
            openTab(urlBarResolver.resolve(arg), false)
        tabs.currentIndex = 0
    }

    Omnibox {
        id: urlBarResolver
        searchEngineName: Config.searchEngineName
        searchTemplate: Config.searchTemplate
    }

    header: ColumnLayout {
        spacing: 0

        TabStrip {
            Layout.fillWidth: true
            tabs: tabs
            currentIndex: tabs.currentIndex
            onActivated: index => tabs.currentIndex = index
            onCloseRequested: index => window.closeTab(index)
            onNewTabRequested: {
                window.openTab("")
                window.focusUrlBar()
            }
        }

        NavigationBar {
            id: navBar
            Layout.fillWidth: true
            view: window.currentView
        }
    }

    StackLayout {
        anchors.fill: parent
        currentIndex: tabs.currentIndex

        Repeater {
            id: views
            model: tabs

            delegate: BrowserTab {
                required property int index
                required property string initialUrl

                profile: window.profile
                url: initialUrl.length > 0 ? initialUrl : "about:blank"

                onTitleChanged: tabs.setProperty(index, "title", title)
                onNewTabRequested: request => {
                    const background = request.destination === WebEngineNewWindowRequest.InNewBackgroundTab
                    request.openIn(window.openTab("", !background))
                }
            }
        }
    }

    // Keyboard shortcuts. "Ctrl" maps to Cmd on macOS automatically.
    Shortcut { sequences: [StandardKey.AddTab]; onActivated: { window.openTab(""); window.focusUrlBar() } }
    Shortcut { sequences: [StandardKey.Close]; onActivated: window.closeTab(tabs.currentIndex) }
    Shortcut { sequences: ["Ctrl+L", "Alt+D", "F6"]; onActivated: window.focusUrlBar() }
    Shortcut { sequences: ["Ctrl+Tab", "Ctrl+PgDown"]; onActivated: window.cycleTab(1) }
    Shortcut { sequences: ["Ctrl+Shift+Tab", "Ctrl+PgUp"]; onActivated: window.cycleTab(-1) }
    Shortcut { sequences: [StandardKey.Refresh, "Ctrl+R"]; onActivated: window.currentView?.reload() }
    Shortcut { sequences: [StandardKey.Back]; onActivated: window.currentView?.goBack() }
    Shortcut { sequences: [StandardKey.Forward]; onActivated: window.currentView?.goForward() }
    Shortcut { sequences: [StandardKey.Quit]; onActivated: Qt.quit() }
}
