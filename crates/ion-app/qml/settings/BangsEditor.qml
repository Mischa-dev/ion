import QtQuick
import Ion

// Your own bangs (`[bangs]`): `!name words` searches the address with {} for
// the words. Each one is saved as its own `bangs.<name>` key, so removing one
// here leaves any set in config.toml alone.
PairList {
    id: editor

    property var setting

    readonly property var bangs: {
        Config.revision
        return Config.value("bangs") || {}
    }

    items: Object.keys(bangs).sort().map(name => ({ name: name, value: bangs[name] }))
    namePrefix: "!"
    namePlaceholder: qsTr("Name, like gh")
    valuePlaceholder: qsTr("Address with {} for the words")
    add: (name, value) => {
        name = name.replace(/^!/, "").toLowerCase()
        if (!/^[a-z0-9_-]+$/.test(name)) {
            error = qsTr("A bang's name is letters, digits, - or _.")
            return false
        }
        if (!value.includes("{}")) {
            error = qsTr("Put {} where your words go.")
            return false
        }
        error = Config.set("bangs." + name, value)
        return error.length === 0
    }
    onRemoveClicked: index => {
        const name = items[index].name
        error = Config.reset("bangs." + name)
        // Still there: config.toml sets it, so turn it off here instead.
        if (error.length === 0 && (Config.value("bangs." + name) || "").length > 0)
            error = qsTr("!%1 is set in config.toml.").arg(name)
    }
}
