import QtQuick
import QtQuick.Controls
import Ion

// Divider between groups of IonMenuItems.
MenuSeparator {
    topPadding: Theme.spacing
    bottomPadding: Theme.spacing
    leftPadding: Theme.spacing * 2
    rightPadding: Theme.spacing * 2
    contentItem: Rectangle {
        implicitHeight: Theme.hairline
        color: Theme.border
    }
}
