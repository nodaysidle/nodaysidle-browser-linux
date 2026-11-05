import QtQuick

// A toolbar button whose glyph is drawn on a fixed 16 px grid, so every icon sits on the same centre line.
Rectangle {
    id: root
    property string icon: "back"
    property bool active: false
    property int cornerRadius: 4
    signal clicked()

    width: 28
    height: 28
    radius: cornerRadius
    color: active ? Qt.alpha(theme.accent, 0.25) : (area.containsMouse ? theme.surface : "transparent")
    border.color: active ? theme.accent : "transparent"

    Canvas {
        anchors.centerIn: parent
        width: 16
        height: 16
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
            }
            c.stroke();
        }
    }

    MouseArea {
        id: area
        anchors.fill: parent
        hoverEnabled: true
        onClicked: root.clicked()
    }
}
