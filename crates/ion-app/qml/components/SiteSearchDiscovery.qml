import QtQuick
import QtWebEngine
import Ion

// Finds the search a site describes for itself (an OpenSearch link in its
// page) once per site, and hands it to `AddressBar.addSiteSearch`, which
// makes it a bang: typing "youtube" then Tab searches YouTube.
QtObject {
    id: root

    readonly property string findLink: "(() => {"
        + " const link = document.querySelector('link[rel~=\"search\"][type=\"application/opensearchdescription+xml\"]');"
        + " return link ? link.href : ''; })()"

    // Call after `view` finished loading `pageUrl`.
    function check(view, pageUrl) {
        const page = pageUrl.toString()
        if (!AddressBar.wantsSiteSearch(page))
            return
        view.runJavaScript(findLink, WebEngineScript.ApplicationWorld, href => {
            if (typeof href !== "string" || !/^https?:\/\//.test(href))
                return
            const xhr = new XMLHttpRequest()
            xhr.onreadystatechange = () => {
                if (xhr.readyState === XMLHttpRequest.DONE && xhr.status === 200)
                    AddressBar.addSiteSearch(page, xhr.responseText)
            }
            xhr.open("GET", href)
            xhr.send()
        })
    }
}
