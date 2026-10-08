import QtQuick
import QtWebEngine
import Ion

// Runs the browser tools `Agent` asks for, on the web views they name.
// Every call is checked with the safety model first (`Agent.authorize`);
// nothing here decides.
QtObject {
    id: tools

    // The window's Repeater of BrowserTab views, one per `Tabs` row.
    required property Repeater views

    function viewForTab(tabId) {
        for (let i = 0; i < views.count; ++i) {
            const view = views.itemAt(i)
            if (view && view.tabId === tabId)
                return view
        }
        return null
    }

    // Read in Ion's own script world, so the page can't see or change it.
    readonly property string readPageScript: "(function () {"
        + " const text = document.body ? document.body.innerText : '';"
        + " return JSON.stringify({ title: document.title, text: text.slice(0, 200000) });"
        + "})()"

    function run(task, call, tool, tab) {
        const fail = error => Agent.toolResult(task, call, false, JSON.stringify({ error: error }))
        switch (tool) {
        case "read_page": {
            const view = viewForTab(tab)
            if (!view) {
                fail(qsTr("the tab was closed"))
                return
            }
            const url = view.url.toString()
            view.runJavaScript(readPageScript, WebEngineScript.ApplicationWorld, result => {
                let page = null
                try { page = JSON.parse(result) } catch (e) {}
                if (!page) {
                    fail(qsTr("the page couldn't be read"))
                    return
                }
                Agent.toolResult(task, call, true, JSON.stringify({ url: url, title: page.title, text: page.text }))
            })
            break
        }
        case "list_tabs": {
            const list = []
            for (let i = 0; i < Tabs.count; ++i)
                list.push({ title: Tabs.titleAt(i), url: Tabs.urlAt(i) })
            Agent.toolResult(task, call, true, JSON.stringify({ tabs: list }))
            break
        }
        default:
            fail(qsTr("Ion has no tool called %1").arg(tool))
        }
    }

    property list<QtObject> connections: [
        Connections {
            target: Agent
            function onToolRequested(task, call, tool, tab) {
                const view = tools.viewForTab(tab)
                const url = view ? view.url.toString() : ""
                // Remember what to run once the person allows it.
                tools.waiting[task + ":" + call] = { tool: tool, tab: tab }
                if (Agent.authorize(task, call, url) === "allow")
                    tools.start(task, call)
            }
            function onToolApproved(task, call) {
                tools.start(task, call)
            }
        },
        // The global stop ends every task, not just their next step.
        Connections {
            target: Safety
            function onStoppedChanged() {
                if (Safety.stopped)
                    Agent.stopAll()
            }
        }
    ]

    property var waiting: ({})
    function start(task, call) {
        const key = task + ":" + call
        const item = waiting[key]
        if (!item)
            return
        delete waiting[key]
        run(task, call, item.tool, item.tab)
    }
}
