import QtQuick
import QtQuick.Controls
import Ion

// The address field. Shows the current page URL; on Enter, hands the typed
// text to `Omnibox` (Rust) to decide between an address and a search.
TextField {
    id: field

    // URL of the page in the current tab.
    property url currentUrl
    // Emitted with the resolved URL when the person presses Enter.
    signal navigate(url target)

    Omnibox { id: omnibox }

    implicitHeight: Theme.urlBarHeight
    color: Theme.text
    placeholderText: qsTr("Search or enter address")
    placeholderTextColor: Theme.textMuted
    selectionColor: Theme.accent
    selectedTextColor: Theme.background
    font.pixelSize: Theme.fontSize
    leftPadding: Theme.spacing * 2
    rightPadding: Theme.spacing * 2
    verticalAlignment: TextInput.AlignVCenter
    selectByMouse: true

    background: Rectangle {
        radius: Theme.radius
        color: Theme.surfaceRaised
        border.width: field.activeFocus ? 1 : 0
        border.color: Theme.accent
    }

    function showUrl() {
        text = currentUrl.toString() === "about:blank" ? "" : currentUrl.toString()
    }

    onCurrentUrlChanged: if (!activeFocus) showUrl()
    onActiveFocusChanged: activeFocus ? selectAll() : showUrl()

    onAccepted: {
        const target = omnibox.resolve(text)
        if (target.toString().length === 0)
            return
        navigate(target)
        focus = false
    }

    Keys.onEscapePressed: {
        showUrl()
        focus = false
    }
}
