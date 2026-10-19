import QtQuick

// Creates its component the first time `get()` is called. For menus and
// dialogs that many sessions never open, so building the window skips them.
Loader {
    active: false

    function get() {
        active = true
        return item
    }
}
