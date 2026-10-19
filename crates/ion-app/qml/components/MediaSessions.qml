import QtQuick
import Ion

// Which tabs are playing, for the now-playing button and the system's media
// controls (MPRIS on Linux). Tabs report through BrowserTab.media; the tab
// that last started playing is `active`, and the system's controls show it
// and steer it. When it has nothing left to play, the next tab that does
// takes over.
QtObject {
    id: sessions

    // The browser window, raised by the system's "show" action.
    required property var browser
    // The picture-in-picture window, for videos' button in the panel.
    property var pip: null

    // Tabs with media, the one that started playing most recently first.
    property var tabs: []
    readonly property var active: tabs.length > 0 ? tabs[0] : null
    readonly property bool anyPlaying: tabs.some(view => view.media && view.media.playing)

    // Tab ids that were playing at their last report, to spot a new start.
    property var wasPlaying: ({})

    // A tab's media changed (or it closed, with `gone`).
    function update(view, gone) {
        const media = gone ? null : view.media
        const playing = !!(media && media.playing)
        const started = playing && !wasPlaying[view.tabId]
        const states = Object.assign({}, wasPlaying)
        if (playing)
            states[view.tabId] = true
        else
            delete states[view.tabId]
        wasPlaying = states

        const others = tabs.filter(tab => tab !== view)
        if (!media)
            tabs = others
        else if (started || tabs.indexOf(view) < 0 && playing)
            tabs = [view].concat(others)
        else if (tabs.indexOf(view) < 0)
            tabs = others.concat([view])
        publish()
    }

    function host(url) {
        const match = /^[a-z][a-z0-9+.-]*:\/\/([^/?#]+)/i.exec(url)
        return match ? match[1].replace(/^www\./, "") : ""
    }

    // What the system's controls show: the page's own track details, else
    // the tab's title and site.
    function publish() {
        const view = active
        const media = view ? view.media : null
        if (!media) {
            Media.publish(false, "", "", "", "", "", 0, 0, false, false, false)
            return
        }
        const url = view.url.toString()
        Media.publish(true, media.title || view.title, media.artist || host(url), media.album,
                      media.artwork, url, media.duration, media.position, media.playing,
                      media.canPrevious, media.canNext)
    }

    // Bring a tab to the front, from the system's controls or Ion's own.
    function show(view) {
        if (!view)
            return
        browser.raise()
        browser.requestActivate()
        const index = Tabs.indexOfTab(view.tabId)
        if (index >= 0)
            Tabs.activate(index)
    }

    property Connections system: Connections {
        target: Media
        function onSystemAction(action, value) {
            if (action === "raise")
                sessions.show(sessions.active)
            else if (sessions.active)
                sessions.active.mediaAction(action, value)
        }
    }

    // The fallback name follows the tab's title.
    property Connections activeTitle: Connections {
        target: sessions.active
        function onTitleChanged() { sessions.publish() }
    }
}
