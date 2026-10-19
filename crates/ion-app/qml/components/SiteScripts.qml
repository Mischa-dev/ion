import QtQuick
import QtWebEngine
import Ion

// Installs `Sites.scripts()` and `Privacy.fingerprintScript()` (privacy signal, per-site user agents, keyboard
// mode and the files in the userscripts folder) on the browser profile, and again whenever the config
// changes or `Sites.reload()` is called. New page loads pick them up.
QtObject {
    id: root

    property WebEngineProfile profile

    function install() {
        if (!profile)
            return
        // The engine's user agent, which Ion's default (set as the
        // profile's own) is made from; per-site ones are set per request.
        profile.httpUserAgent = Sites.setEngineUserAgent(profile.httpUserAgent)
        const scripts = Sites.scripts().map(script => ({
            name: script.name,
            sourceCode: script.sourceCode,
            injectionPoint: WebEngineScript[script.injectionPoint],
            worldId: script.mainWorld ? WebEngineScript.MainWorld : WebEngineScript.ApplicationWorld,
            runsOnSubFrames: true
        }))
        // Fingerprinting protection, before any page script in every frame.
        const fingerprint = Privacy.fingerprintScript()
        if (fingerprint.length > 0)
            scripts.unshift({
                name: "ion-fingerprinting",
                sourceCode: fingerprint,
                injectionPoint: WebEngineScript.DocumentCreation,
                worldId: WebEngineScript.MainWorld,
                runsOnSubFrames: true
            })
        profile.userScripts.collection = scripts
    }

    onProfileChanged: install()

    property Connections configWatch: Connections {
        target: Config
        function onRevisionChanged() { root.install() }
    }
    // The shield's per-site switch also turns fingerprinting protection off.
    property Connections shieldWatch: Connections {
        target: Adblock
        function onFiltersRevisionChanged() { root.install() }
    }
    property Connections sitesWatch: Connections {
        target: Sites
        function onRevisionChanged() { root.install() }
    }
}
