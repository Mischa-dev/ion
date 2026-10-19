import QtQuick
import Ion

// `newTab.shortcuts`: tiles the new tab page always shows first, in order.
PairList {
    id: editor

    property var setting

    readonly property var tiles: {
        Config.revision
        // A list from Config is array-like, not an Array.
        const value = Config.value("newTab.shortcuts")
        return value ? Array.from(value) : []
    }

    function save(list) {
        error = Config.set("newTab.shortcuts", list)
        return error.length === 0
    }

    items: tiles.map(t => ({ name: t.title, value: t.url }))
    namePlaceholder: qsTr("Title")
    valuePlaceholder: qsTr("Address")
    add: (name, value) => {
        const url = /^[a-z]+:/i.test(value) ? value : "https://" + value
        return save(tiles.concat([{ title: name, url: url }]))
    }
    onRemoveClicked: index => {
        const list = tiles.slice()
        list.splice(index, 1)
        save(list)
    }
}
