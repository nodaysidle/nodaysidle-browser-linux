import QtQuick
import QtQuick.Controls

MenuItem {
    id: root
    implicitHeight: visible ? 30 : 0
    height: visible ? implicitHeight : 0
    font.pixelSize: 12

    contentItem: Text {
        text: root.text
        font: root.font
        color: root.enabled ? theme.textStrong : theme.muted
        verticalAlignment: Text.AlignVCenter
        leftPadding: 6
    }

    background: Rectangle {
        color: root.highlighted ? Qt.alpha(theme.accent, 0.22) : "transparent"
        radius: Math.min(theme.radius, 6)
    }
}
