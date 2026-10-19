import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import Ion

// One row of the Settings page: label, help, where the value comes from, and
// the control for the setting's `control.kind` (crates/ion-settings). Values
// are saved with `Config.set`, which writes overrides.toml.
Item {
    id: row

    // A setting from `Preferences.page`: key, label, help, control, value,
    // default, origin ("default" | "base" | "override"), base, selected.
    property var setting
    // The first row of its card has no divider above it.
    property bool first: false

    // A `Control::Action` button was pressed.
    signal action(string action)

    readonly property var control: setting.control
    readonly property string kind: control.kind
    // Lists and checkboxes go under the label; everything else beside it.
    readonly property bool wide: kind === "list" || kind === "flags"
    readonly property bool active: !setting.requires
        || (Config.revision >= 0 && Config.value(setting.requires) === true)
    // Why the last change was refused, if it was.
    property string error: ""

    function save(value) {
        error = Config.set(setting.key, value)
    }

    // Presets write several keys; the first refusal stops the rest.
    function savePreset(index) {
        const values = control.options[index].values
        for (const [key, value] of values) {
            error = Config.set(key, value)
            if (error.length > 0)
                return
        }
    }

    function reset() {
        if (kind === "preset") {
            const keys = new Set()
            control.options.forEach(o => o.values.forEach(([key]) => keys.add(key)))
            keys.forEach(key => Config.reset(key))
        } else {
            error = Config.reset(setting.key)
        }
    }

    // How a value reads in a sentence.
    function describe(value) {
        if (value === true)
            return qsTr("on")
        if (value === false)
            return qsTr("off")
        if (Array.isArray(value))
            return value.length > 0 ? value.join(", ") : qsTr("nothing")
        if (value === "")
            return qsTr("empty")
        const options = control.options
        if (options) {
            const match = options.find(o => o.value === value)
            if (match)
                return match.label
        }
        return String(value)
    }

    implicitHeight: layout.implicitHeight + Theme.spacing * 5
    opacity: active ? 1 : Theme.dimmedOpacity
    enabled: active

    Behavior on opacity { NumberAnimation { duration: Theme.animationMs } }

    Rectangle {
        visible: !row.first
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacing * 3
        anchors.rightMargin: Theme.spacing * 3
        height: Theme.hairline
        color: Theme.border
    }

    ColumnLayout {
        id: layout
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: Theme.spacing * 3
        anchors.rightMargin: Theme.spacing * 3
        spacing: Theme.spacing * 1.5

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacing * 3

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacing / 2

                Text {
                    Layout.fillWidth: true
                    text: row.setting.label
                    color: Theme.text
                    font.pixelSize: Theme.fontSize
                    wrapMode: Text.Wrap
                }
                Text {
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: row.setting.help
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSize - 1
                    wrapMode: Text.Wrap
                    lineHeight: 1.1
                }

                // Where the value comes from, with a way back.
                RowLayout {
                    visible: row.setting.origin !== "default" && row.kind !== "action"
                    spacing: Theme.spacing

                    Text {
                        text: row.setting.origin === "base"
                            ? qsTr("Set in config.toml")
                            : row.setting.base !== null && row.kind !== "preset"
                                ? qsTr("Changed here; config.toml says %1").arg(row.describe(row.setting.base))
                                : qsTr("Changed here")
                        color: row.setting.origin === "base" ? Theme.textMuted : Theme.accent
                        font.pixelSize: Theme.fontSize - 2
                    }
                    Text {
                        visible: row.setting.origin === "override"
                        text: row.setting.base !== null ? qsTr("Use config.toml") : qsTr("Reset")
                        color: resetMouse.containsMouse ? Theme.text : Theme.textMuted
                        font.pixelSize: Theme.fontSize - 2
                        font.underline: true

                        MouseArea {
                            id: resetMouse
                            anchors.fill: parent
                            anchors.margins: -Theme.spacing
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: row.reset()
                        }
                    }
                }

                Text {
                    Layout.fillWidth: true
                    visible: row.error.length > 0
                    text: row.error
                    color: Theme.danger
                    font.pixelSize: Theme.fontSize - 1
                    wrapMode: Text.Wrap
                }
            }

            Loader {
                Layout.alignment: Qt.AlignVCenter | Qt.AlignRight
                active: !row.wide
                sourceComponent: row.kind === "toggle" ? toggleControl
                               : row.kind === "choice" ? choiceControl
                               : row.kind === "preset" ? choiceControl
                               : row.kind === "text" ? textControl
                               : row.kind === "number" ? numberControl
                               : row.kind === "folder" ? folderControl
                               : row.kind === "action" ? actionControl
                               : row.kind === "custom" ? customControl
                               : null
            }
        }

        Loader {
            Layout.fillWidth: true
            active: row.wide
            sourceComponent: row.kind === "list" ? listControl : flagsControl
        }
    }

    // A click anywhere on a switch's row flips it.
    MouseArea {
        anchors.fill: parent
        z: -1
        enabled: row.kind === "toggle"
        cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: row.save(row.setting.value !== true)
    }

    Component {
        id: toggleControl
        IonSwitch {
            checked: row.setting.value === true
            onToggled: row.save(checked)
        }
    }

    // A few short options as a segmented control; more as a menu.
    Component {
        id: choiceControl
        Item {
            id: choice
            readonly property bool preset: row.kind === "preset"
            readonly property var options: row.control.options
            readonly property int selected: preset ? row.setting.selected
                : options.findIndex(o => o.value === row.setting.value)
            readonly property bool segmented: options.length <= 4
                && options.reduce((n, o) => n + o.label.length, 0) <= 40

            function pick(index) {
                if (index === selected)
                    return
                if (preset)
                    row.savePreset(index)
                else
                    row.save(options[index].value)
            }

            implicitWidth: segmented ? segments.implicitWidth : menuButton.implicitWidth
            implicitHeight: Theme.urlBarHeight

            Rectangle {
                id: segments
                visible: choice.segmented
                anchors.fill: parent
                implicitWidth: segmentRow.implicitWidth + Theme.spacing
                radius: Theme.radius
                color: Theme.surfaceRaised

                Row {
                    id: segmentRow
                    anchors.centerIn: parent
                    spacing: 0

                    Repeater {
                        model: choice.segmented ? choice.options : []

                        delegate: Rectangle {
                            id: segment
                            required property var modelData
                            required property int index
                            readonly property bool on: index === choice.selected

                            width: segmentText.implicitWidth + Theme.spacing * 4
                            height: Theme.urlBarHeight - Theme.spacing
                            radius: Math.max(0, Theme.radius - 2)
                            color: on ? Theme.accent : segmentMouse.containsMouse ? Theme.surfaceHover : "transparent"

                            Behavior on color { ColorAnimation { duration: Theme.animationMs } }

                            Text {
                                id: segmentText
                                anchors.centerIn: parent
                                text: segment.modelData.label
                                color: segment.on ? Theme.onAccent : Theme.text
                                font.pixelSize: Theme.fontSize - 1
                            }
                            MouseArea {
                                id: segmentMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: choice.pick(segment.index)
                            }
                        }
                    }
                }
            }

            Rectangle {
                id: menuButton
                visible: !choice.segmented
                anchors.fill: parent
                implicitWidth: Math.max(Theme.tabMaxWidth * 0.8, menuLabel.implicitWidth + Theme.spacing * 7)
                radius: Theme.radius
                color: menuMouse.containsMouse ? Theme.surfaceHover : Theme.surfaceRaised

                Text {
                    id: menuLabel
                    anchors.left: parent.left
                    anchors.leftMargin: Theme.spacing * 2
                    anchors.verticalCenter: parent.verticalCenter
                    text: choice.selected >= 0 ? choice.options[choice.selected].label : qsTr("Custom")
                    color: Theme.text
                    font.pixelSize: Theme.fontSize
                }
                Text {
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.spacing * 2
                    anchors.verticalCenter: parent.verticalCenter
                    text: "▾"
                    color: Theme.textMuted
                    font.pixelSize: Theme.fontSize
                }
                MouseArea {
                    id: menuMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: menu.popup(menuButton, 0, menuButton.height + Theme.spacing / 2)
                }

                IonMenu {
                    id: menu
                    width: Math.max(menuButton.width, Theme.tabMaxWidth)

                    Instantiator {
                        model: choice.segmented ? [] : choice.options
                        delegate: IonMenuItem {
                            required property var modelData
                            required property int index
                            text: modelData.label
                            checkable: true
                            checked: index === choice.selected
                            onTriggered: choice.pick(index)
                        }
                        onObjectAdded: (index, object) => menu.insertItem(index, object)
                        onObjectRemoved: (index, object) => menu.removeItem(object)
                    }
                }
            }
        }
    }

    Component {
        id: textControl
        IonTextField {
            implicitWidth: Theme.tabMaxWidth * 1.3
            text: row.setting.value ?? ""
            placeholderText: row.control.placeholder
            onEditingFinished: if (text !== (row.setting.value ?? "")) row.save(text)
            Keys.onEscapePressed: {
                text = row.setting.value ?? ""
                focus = false
            }
        }
    }

    Component {
        id: numberControl
        RowLayout {
            spacing: Theme.spacing * 2

            SettingSlider {
                id: slider
                visible: row.control.slider
                Layout.preferredWidth: Theme.tabMaxWidth * 0.9
                from: row.control.min
                to: row.control.max
                stepSize: row.control.step
                value: Number(row.setting.value)
                // Saved when let go, not on every step of a drag.
                onPressedChanged: if (!pressed && value !== Number(row.setting.value)) row.save(value)
                onMoved: if (!pressed) row.save(value)
            }
            Text {
                visible: row.control.slider
                Layout.minimumWidth: Theme.iconSize * 3
                text: (slider.pressed ? slider.value : Number(row.setting.value)).toLocaleString(Qt.locale(), "f", row.control.step < 1 ? 2 : 0)
                      + (row.control.unit.length > 0 ? " " + row.control.unit : "")
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize - 1
                font.features: { "tnum": 1 }
                horizontalAlignment: Text.AlignRight
            }
            IonTextField {
                visible: !row.control.slider
                implicitWidth: Theme.iconSize * 5
                text: String(row.setting.value)
                horizontalAlignment: TextInput.AlignRight
                validator: DoubleValidator { bottom: row.control.min; top: row.control.max }
                onEditingFinished: {
                    const value = Number(text)
                    if (acceptableInput && value !== Number(row.setting.value))
                        row.save(value)
                    else
                        text = String(row.setting.value)
                }
            }
            Text {
                visible: !row.control.slider && row.control.unit.length > 0
                text: row.control.unit
                color: Theme.textMuted
                font.pixelSize: Theme.fontSize - 1
            }
        }
    }

    Component {
        id: folderControl
        RowLayout {
            spacing: Theme.spacing
            IonTextField {
                id: folderField
                implicitWidth: Theme.tabMaxWidth
                text: row.setting.value ?? ""
                placeholderText: row.control.placeholder
                onEditingFinished: if (text !== (row.setting.value ?? "")) row.save(text)
            }
            DialogButton {
                text: qsTr("Choose…")
                onClicked: folderDialog.open()
            }
            FolderDialog {
                id: folderDialog
                title: row.setting.label
                onAccepted: {
                    const url = selectedFolder.toString()
                    row.save(decodeURIComponent(url.replace(/^file:\/\//, "")))
                }
            }
        }
    }

    Component {
        id: actionControl
        DialogButton {
            text: row.control.button
            onClicked: row.action(row.control.action)
        }
    }

    // Drawn by qml/settings/<component>.qml, which gets `setting` and may
    // emit `action`.
    Component {
        id: customControl
        Loader {
            source: "qrc:/qt/qml/Ion/qml/settings/" + row.control.component + ".qml"
            onLoaded: {
                if (item.hasOwnProperty("setting"))
                    item.setting = Qt.binding(() => row.setting)
                if (item.action)
                    item.action.connect(row.action)
            }
        }
    }

    // A list of strings as removable chips, plus a field to add one.
    Component {
        id: listControl
        ColumnLayout {
            spacing: Theme.spacing
            readonly property var items: Array.isArray(row.setting.value) ? row.setting.value : []

            Flow {
                id: chips
                Layout.fillWidth: true
                spacing: Theme.spacing
                visible: parent.items.length > 0

                Repeater {
                    model: chips.parent.items
                    delegate: Rectangle {
                        id: chip
                        required property var modelData
                        required property int index
                        height: Theme.tabHeight - Theme.spacing
                        width: chipText.implicitWidth + chipClose.width + Theme.spacing * 4
                        radius: height / 2
                        color: Theme.surfaceRaised

                        Text {
                            id: chipText
                            anchors.left: parent.left
                            anchors.leftMargin: Theme.spacing * 2
                            anchors.verticalCenter: parent.verticalCenter
                            text: chip.modelData
                            color: Theme.text
                            font.pixelSize: Theme.fontSize - 1
                            textFormat: Text.PlainText
                        }
                        Text {
                            id: chipClose
                            anchors.right: parent.right
                            anchors.rightMargin: Theme.spacing * 1.5
                            anchors.verticalCenter: parent.verticalCenter
                            text: "×"
                            color: closeMouse.containsMouse ? Theme.danger : Theme.textMuted
                            font.pixelSize: Theme.fontSize + 1

                            MouseArea {
                                id: closeMouse
                                anchors.fill: parent
                                anchors.margins: -Theme.spacing
                                hoverEnabled: true
                                cursorShape: Qt.PointingHandCursor
                                onClicked: {
                                    const items = chips.parent.items.slice()
                                    items.splice(chip.index, 1)
                                    row.save(items)
                                }
                            }
                        }
                    }
                }
            }
            IonTextField {
                Layout.fillWidth: true
                placeholderText: row.control.placeholder
                onAccepted: {
                    const value = text.trim()
                    if (value.length > 0 && !parent.items.includes(value))
                        row.save(parent.items.concat([value]))
                    text = ""
                }
            }
        }
    }

    // Several values out of a fixed set, as chips that toggle.
    Component {
        id: flagsControl
        Flow {
            id: flags
            spacing: Theme.spacing
            readonly property var items: Array.isArray(row.setting.value) ? row.setting.value : []

            Repeater {
                model: row.control.options
                delegate: Rectangle {
                    id: flag
                    required property var modelData
                    readonly property bool on: flags.items.includes(modelData.value)

                    height: Theme.tabHeight - Theme.spacing
                    width: flagText.implicitWidth + Theme.spacing * 5 + Theme.iconSize
                    radius: height / 2
                    color: on ? Theme.accent : flagMouse.containsMouse ? Theme.surfaceHover : Theme.surfaceRaised

                    Behavior on color { ColorAnimation { duration: Theme.animationMs } }

                    Row {
                        anchors.centerIn: parent
                        spacing: Theme.spacing
                        Text {
                            width: Theme.iconSize * 0.75
                            text: flag.on ? "✓" : "+"
                            color: flag.on ? Theme.onAccent : Theme.textMuted
                            font.pixelSize: Theme.fontSize - 1
                            horizontalAlignment: Text.AlignHCenter
                        }
                        Text {
                            id: flagText
                            text: flag.modelData.label
                            color: flag.on ? Theme.onAccent : Theme.text
                            font.pixelSize: Theme.fontSize - 1
                        }
                    }
                    MouseArea {
                        id: flagMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            const value = flag.modelData.value
                            row.save(flag.on ? flags.items.filter(v => v !== value)
                                             : flags.items.concat([value]))
                        }
                    }
                }
            }
        }
    }
}
