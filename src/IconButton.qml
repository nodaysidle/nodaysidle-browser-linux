import QtQuick
import QtQuick.Controls

// The single button primitive: a 16 px glyph or a short label, a visible focus ring,
// a screen-reader name, and Enter/Space activation. Every toolbar and bar button uses it,
// so the keyboard reaches the same things the mouse does.
Rectangle {
    id: root
    property string icon: "back"
    property string label: ""
    property string accessibleName: ""
    property bool active: false
    property int cornerRadius: 4
    property int buttonSize: 28
    signal clicked()

    width: label.length > 0 ? labelText.implicitWidth + 24 : buttonSize
    height: buttonSize
    radius: cornerRadius
    color: active ? Qt.alpha(theme.accent, 0.25) : (area.containsMouse ? theme.surface : "transparent")
    border.width: 1
    border.color: activeFocus || active ? theme.accent : "transparent"

    activeFocusOnTab: true
    Accessible.role: Accessible.Button
    Accessible.name: root.accessibleName.length > 0 ? root.accessibleName : (root.label.length > 0 ? root.label : root.icon)
    Accessible.onPressAction: root.clicked()

    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) {
            root.clicked();
            event.accepted = true;
        }
    }

    Canvas {
        anchors.centerIn: parent
        width: 16
        height: 16
        visible: root.label.length === 0
        property color ink: root.active ? theme.accent : theme.textStrong
        property string glyph: root.icon
        onInkChanged: requestPaint()
        onGlyphChanged: requestPaint()
        onPaint: {
            var c = getContext("2d");
            c.reset();
            c.strokeStyle = ink;
            c.lineWidth = 1.7;
            c.lineCap = "round";
            c.lineJoin = "round";
            c.beginPath();
            switch (glyph) {
            case "back":
                c.moveTo(10, 3); c.lineTo(5, 8); c.lineTo(10, 13);
                break;
            case "forward":
                c.moveTo(6, 3); c.lineTo(11, 8); c.lineTo(6, 13);
                break;
            case "reload":
                c.arc(8, 8, 5, -0.25 * Math.PI, 1.25 * Math.PI);
                c.moveTo(11.6, 1.6); c.lineTo(11.54, 4.46); c.lineTo(8.7, 4.4);
                break;
            case "stop":
                c.moveTo(4, 4); c.lineTo(12, 12); c.moveTo(12, 4); c.lineTo(4, 12);
                break;
            case "home":
                c.moveTo(2, 8); c.lineTo(8, 2.5); c.lineTo(14, 8);
                c.moveTo(4, 7); c.lineTo(4, 13.5); c.lineTo(12, 13.5); c.lineTo(12, 7);
                break;
            case "key":
                c.arc(4.5, 11.5, 3.2, 0, Math.PI * 2);
                c.moveTo(7, 9); c.lineTo(14, 2); c.moveTo(11.5, 4.5); c.lineTo(13.5, 6.5);
                break;
            case "shield":
                c.moveTo(8, 2.2);
                c.lineTo(13, 4);
                c.lineTo(13, 8);
                c.bezierCurveTo(13, 11.2, 10.9, 13.2, 8, 14);
                c.bezierCurveTo(5.1, 13.2, 3, 11.2, 3, 8);
                c.lineTo(3, 4);
                c.closePath();
                break;
            case "plus":
                c.moveTo(8, 3); c.lineTo(8, 13); c.moveTo(3, 8); c.lineTo(13, 8);
                break;
            case "close":
                c.moveTo(4.5, 4.5); c.lineTo(11.5, 11.5); c.moveTo(11.5, 4.5); c.lineTo(4.5, 11.5);
                break;
            }
            c.stroke();
        }
    }

    Text {
        id: labelText
        anchors.centerIn: parent
        visible: root.label.length > 0
        text: root.label
        font.pixelSize: 12
        font.bold: true
        color: root.active ? theme.accent : theme.textStrong
    }

    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        onClicked: {
            root.forceActiveFocus();
            root.clicked();
        }
    }

    ToolTip.visible: area.containsMouse && root.accessibleName.length > 0
    ToolTip.delay: 600
    ToolTip.text: root.accessibleName
}
