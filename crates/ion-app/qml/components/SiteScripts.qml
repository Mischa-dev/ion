import QtQuick
import QtWebEngine
import Ion

// Installs `Sites.scripts()` (per-site CSS from config and the files in the
// userscripts folder) on the browser profile, and again whenever the config
// changes or `Sites.reload()` is called. New page loads pick them up.
QtObject {
    id: root

    property WebEngineProfile profile

    function install() {
        if (!profile)
            return
        profile.userScripts.collection = Sites.scripts().map(script => ({
            name: script.name,
            sourceCode: script.sourceCode,
            injectionPoint: WebEngineScript[script.injectionPoint],
            worldId: script.mainWorld ? WebEngineScript.MainWorld : WebEngineScript.ApplicationWorld,
            runsOnSubFrames: true
        }))
    }

    onProfileChanged: install()

    property Connections configWatch: Connections {
        target: Config
        function onRevisionChanged() { root.install() }
    }
    property Connections sitesWatch: Connections {
        target: Sites
        function onRevisionChanged() { root.install() }
    }
}
