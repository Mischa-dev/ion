import QtQuick
import QtWebEngine
import Ion

// Hides ad elements in one tab with the filter lists' element-hiding (##)
// rules. Site-specific rules go in before the page's own scripts run; generic
// rules (`##.ad-banner`) need the page's classes and ids, so the page is
// scanned after it loads and a few more times for late-loading ads.
// Everything runs in Ion's isolated JavaScript world, out of the page's reach.
Item {
    id: root

    required property WebEngineView view

    // Rescans after a load, in ms; ads often arrive after the page.
    readonly property var rescanDelays: [1000, 3000, 8000]
    property int rescans: 0
    // Whether the current page got its site-specific style sheet at creation.
    property bool prepared: false

    visible: false

    // A constructed style sheet: unlike a <style> element it isn't subject to
    // the page's Content Security Policy, and it needs no <html> element, so
    // it works at document creation too.
    function styleScript(css) {
        return "(function (css) {"
            + " var sheet = new CSSStyleSheet();"
            + " sheet.replaceSync(css);"
            + " document.adoptedStyleSheets = document.adoptedStyleSheets.concat([sheet]);"
            + "})(" + JSON.stringify(css) + ");"
    }

    // Classes and ids not reported before on this page; `reset` reports all.
    function scanScript(reset) {
        return "(function (reset) {"
            + " if (reset || !window.__ionAdblockSeen)"
            + "   window.__ionAdblockSeen = { c: new Set(), i: new Set() };"
            + " var seen = window.__ionAdblockSeen;"
            + " var classes = [], ids = [];"
            + " var els = document.querySelectorAll('[class],[id]');"
            + " for (var k = 0; k < els.length; k++) {"
            + "   var e = els[k];"
            + "   if (e.id && !seen.i.has(e.id)) { seen.i.add(e.id); ids.push(e.id); }"
            + "   var cl = e.classList;"
            + "   for (var j = 0; j < cl.length; j++) {"
            + "     if (!seen.c.has(cl[j])) { seen.c.add(cl[j]); classes.push(cl[j]); }"
            + "   }"
            + " }"
            + " return { classes: classes, ids: ids };"
            + "})(" + (reset ? "true" : "false") + ");"
    }

    // Register the page's style sheet before the new document is created.
    function prepare(url) {
        const scripts = root.view.userScripts
        for (const old of scripts.find("ion-adblock"))
            scripts.remove(old)
        const css = Adblock.pageCss(url)
        root.prepared = css !== ""
        if (!root.prepared)
            return
        const script = WebEngine.script()
        script.name = "ion-adblock"
        script.sourceCode = root.styleScript(css)
        script.injectionPoint = WebEngineScript.DocumentCreation
        script.worldId = WebEngineScript.ApplicationWorld
        scripts.insert(script)
    }

    // The site-specific style sheet for a page that is already there.
    function applyPageCss() {
        const css = Adblock.pageCss(root.view.url)
        root.prepared = css !== ""
        if (root.prepared)
            root.view.runJavaScript(root.styleScript(css), WebEngineScript.ApplicationWorld)
    }

    function scan(reset) {
        const url = root.view.url
        root.view.runJavaScript(root.scanScript(reset), WebEngineScript.ApplicationWorld, found => {
            if (!found || (found.classes.length === 0 && found.ids.length === 0))
                return
            const css = Adblock.genericCss(url, found.classes, found.ids)
            if (css !== "" && root.view.url === url)
                root.view.runJavaScript(root.styleScript(css), WebEngineScript.ApplicationWorld)
        })
    }

    Connections {
        target: root.view
        function onLoadingChanged(info) {
            if (info.status === WebEngineView.LoadStartedStatus) {
                rescan.stop()
                root.prepare(info.url)
            } else if (info.status === WebEngineView.LoadSucceededStatus) {
                root.rescans = 0
                if (!root.prepared)
                    root.applyPageCss()
                root.scan(false)
                rescan.interval = root.rescanDelays[0]
                rescan.start()
            }
        }
    }

    // Lists that finish loading after the page did: apply them to it now,
    // scanning every class and id again since earlier scans matched nothing.
    Connections {
        target: Adblock
        function onReadyChanged() {
            // A page still loading gets them when it finishes.
            if (!Adblock.ready || root.view.loading || root.view.url.toString() === "")
                return
            root.applyPageCss()
            root.scan(true)
        }
    }

    Timer {
        id: rescan
        onTriggered: {
            root.scan(false)
            root.rescans += 1
            if (root.rescans < root.rescanDelays.length) {
                interval = root.rescanDelays[root.rescans] - root.rescanDelays[root.rescans - 1]
                start()
            }
        }
    }
}
