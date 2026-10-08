import QtQuick
import QtWebEngine
import Ion

// Runs the browser tools `Agent` asks for, on the web views they name.
// Every call is checked with the safety model first (`Agent.authorize`);
// nothing here decides.
//
// Page scripts run in Ion's own script world, so the page can't see or
// change them, and act through the page structure: they never move the
// person's focus, cursor or scroll position.
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

    // Shared by the scripts below: what an element is, and whether clicking
    // it sends a form.
    readonly property string helpers: `
        const ionRole = el => {
            const role = el.getAttribute('role');
            if (role) return role;
            const tag = el.tagName.toLowerCase();
            const type = (el.getAttribute('type') || 'text').toLowerCase();
            if (tag === 'a') return 'link';
            if (tag === 'select') return 'dropdown';
            if (tag === 'textarea') return 'textbox';
            if (tag === 'input') {
                if (['button', 'submit', 'reset', 'image'].includes(type)) return 'button';
                if (type === 'checkbox' || type === 'radio') return type;
                if (type === 'file') return 'file picker';
                if (type === 'range') return 'slider';
                return 'textbox';
            }
            if (el.isContentEditable) return 'editor';
            return 'button';
        };
        const ionSubmits = el => {
            if (!el.form) return false;
            const tag = el.tagName.toLowerCase();
            const type = (el.getAttribute('type') || '').toLowerCase();
            return (tag === 'button' && (type === '' || type === 'submit'))
                || (tag === 'input' && (type === 'submit' || type === 'image'));
        };
        const ionSensitive = el => {
            const type = (el.getAttribute('type') || '').toLowerCase();
            const auto = (el.getAttribute('autocomplete') || '').toLowerCase();
            return type === 'password'
                || /(^|\\s)(cc-|current-password|new-password|one-time-code)/.test(auto)
                || /card.?num|cvc|cvv|iban/i.test(el.name || '');
        };
        const ionElement = id => {
            const el = (window.__ionAgentElements || [])[id - 1];
            return el && el.isConnected ? el : null;
        };
        const ionResult = value => JSON.stringify(value);
    `

    readonly property string readPageScript: "(function () {" + helpers + `
        const label = el => {
            const aria = el.getAttribute('aria-label');
            if (aria) return aria;
            if (el.labels && el.labels.length) {
                // A label wrapped around its field also holds the field's text.
                const copy = el.labels[0].cloneNode(true);
                copy.querySelectorAll('input, select, textarea, button').forEach(n => n.remove());
                return copy.textContent;
            }
            const by = el.getAttribute('aria-labelledby');
            const labelledBy = by && document.getElementById(by.split(' ')[0]);
            if (labelledBy) return labelledBy.innerText;
            const tag = el.tagName.toLowerCase();
            if (tag === 'input' && ionRole(el) === 'button') return el.value;
            if (tag === 'select') return el.name || '';
            return el.innerText || el.placeholder || el.title || el.alt || el.name || '';
        };
        const value = el => {
            if (ionSensitive(el)) return '';
            const tag = el.tagName.toLowerCase();
            const role = ionRole(el);
            if (tag === 'select') return el.selectedOptions[0] ? el.selectedOptions[0].text : '';
            if (role === 'checkbox' || role === 'radio') return el.checked ? 'checked' : '';
            if (tag === 'textarea' || role === 'textbox') return el.value || '';
            return '';
        };
        const shown = el => {
            const box = el.getBoundingClientRect();
            if (box.width === 0 && box.height === 0) return false;
            const style = getComputedStyle(el);
            return style.visibility !== 'hidden' && style.display !== 'none';
        };
        const found = document.querySelectorAll('a[href], button, input:not([type=hidden]), select, '
            + 'textarea, [role=button], [role=link], [role=checkbox], [role=tab], [role=menuitem], '
            + '[role=switch], [contenteditable=""], [contenteditable=true]');
        const kept = [];
        const elements = [];
        for (const el of found) {
            if (kept.length >= 300) break;
            if (el.disabled || !shown(el)) continue;
            kept.push(el);
            elements.push({ id: kept.length, role: ionRole(el), label: (label(el) || '').trim().slice(0, 200),
                value: (value(el) || '').slice(0, 200), submits: ionSubmits(el), sensitive: ionSensitive(el) });
        }
        window.__ionAgentElements = kept;
        const text = document.body ? document.body.innerText : '';
        return ionResult({ title: document.title, text: text.slice(0, 200000), elements: elements });
    })()`

    function clickScript(id, allowSubmit) {
        return "(function (id, allowSubmit) {" + helpers + `
            const el = ionElement(id);
            if (!el) return ionResult({ error: 'element [' + id + '] is gone; read the page again' });
            if (el.disabled) return ionResult({ error: 'element [' + id + '] is disabled' });
            if (ionSubmits(el) && !allowSubmit)
                return ionResult({ error: 'that would send the form now; read the page again' });
            el.click();
            return ionResult({});
        })(` + id + ", " + (allowSubmit ? "true" : "false") + ")"
    }

    function fillScript(id, text) {
        return "(function (id, text) {" + helpers + `
            const el = ionElement(id);
            if (!el) return ionResult({ error: 'element [' + id + '] is gone; read the page again' });
            if (el.disabled || el.readOnly) return ionResult({ error: 'that field can\\'t be changed' });
            // The person's input comes first.
            if (document.hasFocus() && document.activeElement === el)
                return ionResult({ error: 'the person is typing in that field; leave it to them' });
            const tag = el.tagName.toLowerCase();
            const role = ionRole(el);
            if (tag === 'select') {
                const want = text.trim().toLowerCase();
                const option = Array.from(el.options).find(o =>
                    o.text.trim().toLowerCase() === want || o.value === text);
                if (!option) {
                    const names = Array.from(el.options).map(o => o.text.trim()).join(', ');
                    return ionResult({ error: 'no option "' + text + '"; the options are ' + names });
                }
                el.value = option.value;
            } else if (tag === 'textarea' || (tag === 'input' && role === 'textbox')) {
                // The prototype's setter, so pages that track values (React)
                // see the change.
                const proto = tag === 'input' ? HTMLInputElement.prototype : HTMLTextAreaElement.prototype;
                Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, text);
            } else {
                return ionResult({ error: 'a ' + role + ' can\\'t be filled in; use click' });
            }
            el.dispatchEvent(new Event('input', { bubbles: true }));
            el.dispatchEvent(new Event('change', { bubbles: true }));
            return ionResult({});
        })(` + id + ", " + JSON.stringify(text) + ")"
    }

    // Calls `done` once the view has finished loading whatever the last
    // action started, or after `minWait` ms if nothing started loading.
    property Component settler: Timer {
        property var view
        property var done
        property int minWait: 300
        property int waited: 0
        property bool sawLoading: false
        interval: 100
        repeat: true
        running: true
        onTriggered: {
            waited += interval
            if (view && view.loading)
                sawLoading = true
            const settled = !view || (!view.loading && (sawLoading || waited >= minWait))
            if (settled || waited >= 15000) {
                stop()
                done()
                destroy()
            }
        }
    }

    function afterSettling(view, minWait, done) {
        settler.createObject(tools, { view: view, minWait: minWait, done: done })
    }

    function run(task, call, tool, tab) {
        const fail = error => Agent.toolResult(task, call, false, JSON.stringify({ error: error }))
        const ok = payload => Agent.toolResult(task, call, true, JSON.stringify(payload))
        const view = viewForTab(tab)
        if (!view && tool !== "list_tabs" && tool !== "open_tab") {
            fail(qsTr("the tab was closed"))
            return
        }
        const details = JSON.parse(Agent.callJson(task, call) || "{}")
        // Runs `script` and reports what it returned, after the page settles.
        const runScript = (script, settle) => {
            view.runJavaScript(script, WebEngineScript.ApplicationWorld, result => {
                let out = null
                try { out = JSON.parse(result) } catch (e) {}
                if (!out)
                    fail(qsTr("the page didn't answer"))
                else if (out.error)
                    fail(out.error)
                else if (settle)
                    afterSettling(view, 400, () => ok(out))
                else
                    ok(out)
            })
        }
        switch (tool) {
        case "read_page": {
            const url = view.url.toString()
            view.runJavaScript(readPageScript, WebEngineScript.ApplicationWorld, result => {
                let page = null
                try { page = JSON.parse(result) } catch (e) {}
                if (!page) {
                    fail(qsTr("the page couldn't be read"))
                    return
                }
                ok({ url: url, title: page.title, text: page.text, elements: page.elements })
            })
            break
        }
        case "list_tabs": {
            const list = []
            for (let i = 0; i < Tabs.count; ++i)
                list.push({ title: Tabs.titleAt(i), url: Tabs.urlAt(i) })
            ok({ tabs: list })
            break
        }
        case "click":
            runScript(clickScript(details.element, details.allowSubmit), true)
            break
        case "fill":
            runScript(fillScript(details.element, details.text), false)
            break
        case "open_tab": {
            // Next to the task's other tabs, without switching to it.
            let after = -1
            for (let i = 0; i < views.count; ++i) {
                if (views.itemAt(i) && views.itemAt(i).tabId === details.after)
                    after = i
            }
            const index = Tabs.openTabAt(after >= 0 ? after + 1 : Tabs.count, details.url, false)
            const opened = views.itemAt(index)
            if (!opened) {
                fail(qsTr("the tab didn't open"))
                return
            }
            afterSettling(opened, 1000, () => ok({ tabId: opened.tabId, url: opened.url.toString(), title: opened.title }))
            break
        }
        case "go_to":
            view.url = details.url
            afterSettling(view, 1000, () => ok({ url: view.url.toString(), title: view.title }))
            break
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
