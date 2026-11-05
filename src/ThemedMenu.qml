import QtQuick
import QtQuick.Controls

Menu {
    id: root
    padding: 4

    background: Rectangle {
        implicitWidth: 200
        color: theme.surface
        border.color: theme.line
        radius: Math.min(theme.radius, 8)
    }
}
