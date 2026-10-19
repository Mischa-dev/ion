import QtQuick

// A piece of a BrowserTab's per-page UI, created once the tab is live (see
// BrowserTab.live). A visual piece is handed to the tab itself, so it lays
// out and stacks exactly as if it were declared there.
Loader {
    id: part

    readonly property Item tab: parent

    active: tab.live
    onLoaded: {
        if (item instanceof Item)
            item.parent = part.tab
    }
}
