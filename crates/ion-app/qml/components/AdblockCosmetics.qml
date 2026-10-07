import QtQuick
import QtWebEngine
import Ion

// Hides ad elements in one tab with the filter lists' element-hiding (##)
// rules. Site-specific rules go in before the page's own scripts run; generic
// rules (`##.ad-banner`) need the page's classes and ids, so the page is
// scanned after it loads, and the elements it adds later are scanned as
// they arrive.
// Everything runs in Ion's isolated JavaScript world, out of the page's reach.
Item {
    id: root

    required property WebEngineView view

    // How often elements the page added since the last scan are checked
    // against the generic rules, in ms.
    readonly property int scanInterval: 1000
    // The URL and filter revision the current page's site-specific style
    // sheet was made for; it is replaced when either no longer matches.
    property string preparedFor: ""

    visible: false

    function key(url) {
        return Adblock.filtersRevision + " " + url
    }

    // How often Ion's sheets are put back if the page replaced the adopted
    // sheet list or emptied a sheet (both are shared with the page's world),
    // in ms.
    readonly property int keepInterval: 1000

    // Set (or, with `append`, extend) one of Ion's style sheets on the page.
    // Constructed sheets aren't subject to the page's Content Security Policy
    // and need no <html> element, so this works at document creation too.
    function sheetScript(kind, css, append) {
        return "(function (kind, css, append) {"
            + " var ion = window.__ionAdblock || (window.__ionAdblock = { sheets: {}, text: {}, rules: {} });"
            + " if (!ion.keeper) ion.keeper = setInterval(function () {"
            + "   var list = document.adoptedStyleSheets, missing = [];"
            + "   for (var k in ion.sheets) {"
            + "     var s = ion.sheets[k];"
            + "     if (s.cssRules.length !== ion.rules[k]) s.replaceSync(ion.text[k]);"
            + "     if (list.indexOf(s) < 0) missing.push(s);"
            + "   }"
            + "   if (missing.length) document.adoptedStyleSheets = list.concat(missing);"
            + " }, " + root.keepInterval + ");"
            + " if (append) css = (ion.text[kind] || '') + css;"
            + " var old = ion.sheets[kind];"
            + " var sheets = document.adoptedStyleSheets.filter(function (s) { return s !== old; });"
            + " delete ion.sheets[kind]; ion.text[kind] = css;"
            + " if (css) {"
            + "   var sheet = new CSSStyleSheet(); sheet.replaceSync(css);"
            + "   ion.sheets[kind] = sheet; ion.rules[kind] = sheet.cssRules.length; sheets.push(sheet);"
            + " }"
            + " document.adoptedStyleSheets = sheets;"
            + "})(" + JSON.stringify(kind) + ", " + JSON.stringify(css) + ", " + append + ");"
    }

    // Classes and ids not reported before on this page; `reset` reports all.
    // The first scan covers the whole document and starts watching it; later
    // ones look only at elements added or re-classed since.
    function scanScript(reset) {
        return "(function (reset) {"
            + " var ion = window.__ionAdblock || (window.__ionAdblock = { sheets: {}, text: {}, rules: {} });"
            + " var roots = ion.changed;"
            + " if (reset || !ion.seen || !ion.observer) {"
            + "   ion.seen = { c: new Set(), i: new Set() }; roots = [document];"
            + " }"
            + " if (!ion.observer) {"
            + "   ion.observer = new MutationObserver(function (records) {"
            + "     for (var r = 0; r < records.length; r++) {"
            + "       var rec = records[r];"
            + "       if (rec.type === 'attributes') ion.changed.push(rec.target);"
            + "       else for (var n = 0; n < rec.addedNodes.length; n++)"
            + "         if (rec.addedNodes[n].nodeType === 1) ion.changed.push(rec.addedNodes[n]);"
            + "     }"
            + "   });"
            + "   ion.observer.observe(document, { childList: true, subtree: true, attributes: true, attributeFilter: ['class', 'id'] });"
            + " }"
            + " ion.changed = [];"
            + " if (!roots || roots.length === 0) return null;"
            + " var seen = ion.seen, classes = [], ids = [];"
            + " function add(e) {"
            + "   if (e.id && !seen.i.has(e.id)) { seen.i.add(e.id); ids.push(e.id); }"
            + "   var cl = e.classList;"
            + "   if (cl) for (var j = 0; j < cl.length; j++)"
            + "     if (!seen.c.has(cl[j])) { seen.c.add(cl[j]); classes.push(cl[j]); }"
            + " }"
            + " for (var k = 0; k < roots.length; k++) {"
            + "   if (roots[k].nodeType === 1) add(roots[k]);"
            + "   var els = roots[k].querySelectorAll('[class],[id]');"
            + "   for (var m = 0; m < els.length; m++) add(els[m]);"
            + " }"
            + " return { classes: classes, ids: ids };"
            + "})(" + reset + ");"
    }

    function run(script) {
        root.view.runJavaScript(script, WebEngineScript.ApplicationWorld)
    }

    // Register the page's style sheet before the new document is created.
    function prepare(url) {
        const scripts = root.view.userScripts
        for (const old of scripts.find("ion-adblock"))
            scripts.remove(old)
        root.preparedFor = root.key(url)
        const css = Adblock.pageCss(url)
        if (css === "")
            return
        const script = WebEngine.script()
        script.name = "ion-adblock"
        script.sourceCode = root.sheetScript("page", css, false)
        script.injectionPoint = WebEngineScript.DocumentCreation
        script.worldId = WebEngineScript.ApplicationWorld
        scripts.insert(script)
    }

    // Bring a loaded page's site-specific sheet up to date, e.g. after a
    // redirect to another site or a list update.
    function refreshPageSheet() {
        const url = root.view.url.toString()
        if (root.preparedFor === root.key(url))
            return false
        root.preparedFor = root.key(url)
        root.run(root.sheetScript("page", Adblock.pageCss(url), false))
        return true
    }

    // Check new classes and ids against the generic rules. `reset` rescans
    // the whole page and swaps the generic sheet in one go, so rules that
    // still apply never flicker off.
    function scan(reset) {
        const url = root.view.url
        root.view.runJavaScript(root.scanScript(reset), WebEngineScript.ApplicationWorld, found => {
            const any = found && (found.classes.length > 0 || found.ids.length > 0)
            if (!any && !reset)
                return
            const css = any ? Adblock.genericCss(url, found.classes, found.ids) : ""
            if ((reset || css !== "") && root.view.url === url)
                root.run(root.sheetScript("generic", css, !reset))
        })
    }

    // Redo the loaded page with the current rules and URL.
    function redo() {
        if (root.view.loading || root.view.url.toString() === "")
            return
        if (root.refreshPageSheet())
            root.scan(true)
    }

    Connections {
        target: root.view
        function onLoadingChanged(info) {
            if (info.status === WebEngineView.LoadStartedStatus) {
                rescan.stop()
                root.prepare(info.url)
            } else if (info.status === WebEngineView.LoadSucceededStatus) {
                root.refreshPageSheet()
                root.scan(false)
                rescan.start()
            }
        }
        // history.pushState() and fragment links change the URL without a
        // load, and with it which exceptions apply.
        function onUrlChanged() {
            root.redo()
        }
    }

    // New lists, no lists or a switch changed. A page still loading catches
    // up when it finishes.
    Connections {
        target: Adblock
        function onFiltersRevisionChanged() {
            root.redo()
        }
    }

    Timer {
        id: rescan
        interval: root.scanInterval
        repeat: true
        onTriggered: root.scan(false)
    }
}
