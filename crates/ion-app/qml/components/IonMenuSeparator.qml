import QtQuick
import QtQuick.Controls
import Ion

// Divider between groups of IonMenuItems.
MenuSeparator {
    topPadding: Theme.spacing / 2
    bottomPadding: Theme.spacing / 2
    leftPadding: Theme.spacing
    rightPadding: Theme.spacing
    contentItem: Rectangle {
        implicitHeight: Theme.hairline
        color: Theme.border
    }
}
