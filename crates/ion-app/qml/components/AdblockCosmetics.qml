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

    visible: false

    function styleScript(css) {
        // At document creation there is no <html> element yet; add the
        // style as soon as the parser creates it.
        return "(function (css) {"
            + " function add() {"
            + "   var s = document.createElement('style');"
            + "   s.className = 'ion-adblock'; s.textContent = css;"
            + "   (document.head || document.documentElement).appendChild(s);"
            + " }"
            + " if (document.documentElement) { add(); return; }"
            + " new MutationObserver(function (_, observer) {"
            + "   if (document.documentElement) { observer.disconnect(); add(); }"
            + " }).observe(document, { childList: true });"
            + "})(" + JSON.stringify(css) + ");"
    }

    // Classes and ids not reported before on this page.
    readonly property string scanScript: "(function () {"
        + " var seen = window.__ionAdblockSeen"
        + "     || (window.__ionAdblockSeen = { c: new Set(), i: new Set() });"
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
        + "})();"

    // Register the page's style sheet before the new document is created.
    function prepare(url) {
        const scripts = root.view.userScripts
        for (const old of scripts.find("ion-adblock"))
            scripts.remove(old)
        const css = Adblock.pageCss(url)
        if (css === "")
            return
        const script = WebEngine.script()
        script.name = "ion-adblock"
        script.sourceCode = root.styleScript(css)
        script.injectionPoint = WebEngineScript.DocumentCreation
        script.worldId = WebEngineScript.ApplicationWorld
        scripts.insert(script)
    }

    function scan() {
        const url = root.view.url
        root.view.runJavaScript(root.scanScript, WebEngineScript.ApplicationWorld, found => {
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
                root.scan()
                rescan.interval = root.rescanDelays[0]
                rescan.start()
            }
        }
    }

    Timer {
        id: rescan
        onTriggered: {
            root.scan()
            root.rescans += 1
            if (root.rescans < root.rescanDelays.length) {
                interval = root.rescanDelays[root.rescans] - root.rescanDelays[root.rescans - 1]
                start()
            }
        }
    }
}
