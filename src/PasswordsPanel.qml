import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts

// Saved logins, plus importing from other browsers. Opened with Ctrl+Shift+K.
Rectangle {
    id: root
    property int cornerRadius: 4
    property var logins: []
    property var sources: []
    property var revealed: ({})
    property string importResult: ""
    property bool importFailed: false

    signal closeRequested()

    color: theme.surface
    border.color: theme.line

    function refresh() {
        logins = vault.list(search.text);
    }

    function open() {
        search.text = "";
        revealed = ({});
        importResult = "";
        sources = importer.sources();
        refresh();
        search.forceActiveFocus();
    }

    function showResult(result) {
        importFailed = !result.ok;
        importResult = result.message;
        backend.notice(result.message);
        refresh();
    }

    Connections {
        target: vault
        function onChanged() { root.refresh(); }
    }

    FileDialog {
        id: csvDialog
        title: "Import passwords from CSV"
        nameFilters: ["Passwords export (*.csv)", "All files (*)"]
        onAccepted: root.showResult(importer.importPasswordsCsv(selectedFile.toString()))
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 10

        RowLayout {
            Layout.fillWidth: true

            Text {
                text: "Passwords"
                font.pixelSize: 15
                font.bold: true
                color: theme.textStrong
                Layout.fillWidth: true
            }

            Text {
                text: "×"
                font.pixelSize: 18
                color: theme.muted
                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -4
                    onClicked: root.closeRequested()
                }
            }
        }

        Text {
            Layout.fillWidth: true
            visible: !vault.available
            wrapMode: Text.WordWrap
            text: vault.problem
            color: theme.red
            font.pixelSize: 12
        }

        Text {
            Layout.fillWidth: true
            visible: vault.available
            wrapMode: Text.WordWrap
            text: "Logins are saved on this computer, encrypted, and filled in when you return to a site."
            color: theme.muted
            font.pixelSize: 11
        }

        TextField {
            id: search
            Layout.fillWidth: true
            placeholderText: "Search passwords"
            placeholderTextColor: theme.muted
            color: theme.textStrong
            font.pixelSize: 12
            selectByMouse: true
            background: Rectangle {
                radius: root.cornerRadius
                color: theme.chrome
                border.color: search.activeFocus ? theme.accent : theme.line
            }
            onTextChanged: root.refresh()
        }

        Text {
            Layout.fillWidth: true
            visible: root.logins.length === 0 && vault.available
            text: search.text.length > 0 ? "Nothing matches." : "No saved passwords yet. Sign in to a site and choose Save, or import below."
            wrapMode: Text.WordWrap
            color: theme.muted
            font.pixelSize: 12
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.logins
            spacing: 2

            delegate: Rectangle {
                id: row
                required property var modelData
                readonly property bool shown: root.revealed[modelData.id] !== undefined
                width: ListView.view.width
                height: 48
                radius: Math.min(root.cornerRadius, 6)
                color: rowHover.hovered ? Qt.alpha(theme.accent, 0.12) : "transparent"

                HoverHandler { id: rowHover }

                Column {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.right: actions.left
                    anchors.leftMargin: 8
                    anchors.rightMargin: 6

                    Text {
                        width: parent.width
                        text: row.modelData.origin.replace(/^https?:\/\//, "")
                        elide: Text.ElideRight
                        color: theme.textStrong
                        font.pixelSize: 12
                    }
                    Text {
                        width: parent.width
                        text: row.shown ? row.modelData.username + "   " + root.revealed[row.modelData.id]
                                        : (row.modelData.username.length > 0 ? row.modelData.username : "no username")
                        elide: Text.ElideRight
                        color: row.shown ? theme.accent : theme.muted
                        font.pixelSize: 11
                    }
                }

                Row {
                    id: actions
                    anchors.right: parent.right
                    anchors.rightMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 10
                    visible: rowHover.hovered

                    Repeater {
                        model: [
                            { label: row.shown ? "Hide" : "Show", act: "toggle" },
                            { label: "Copy", act: "copy" },
                            { label: "Delete", act: "delete" }
                        ]

                        Text {
                            required property var modelData
                            text: modelData.label
                            font.pixelSize: 11
                            font.bold: true
                            color: modelData.act === "delete" ? theme.red : theme.accent

                            MouseArea {
                                anchors.fill: parent
                                anchors.margins: -4
                                onClicked: {
                                    const id = row.modelData.id;
                                    if (modelData.act === "toggle") {
                                        const next = Object.assign({}, root.revealed);
                                        if (row.shown) delete next[id]; else next[id] = vault.reveal(id);
                                        root.revealed = next;
                                    } else if (modelData.act === "copy") {
                                        backend.copyText(vault.reveal(id));
                                        backend.notice("Password copied");
                                    } else {
                                        vault.remove(id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Rectangle { Layout.fillWidth: true; height: 1; color: theme.line }

        Text {
            text: "Import from another browser"
            font.pixelSize: 13
            font.bold: true
            color: theme.textStrong
        }

        Text {
            Layout.fillWidth: true
            visible: root.sources.length === 0
            text: "No other browser was found on this computer."
            wrapMode: Text.WordWrap
            color: theme.muted
            font.pixelSize: 11
        }

        Repeater {
            model: root.sources

            Rectangle {
                id: source
                required property var modelData
                Layout.fillWidth: true
                height: 30
                radius: Math.min(root.cornerRadius, 6)
                color: sourceHover.containsMouse ? Qt.alpha(theme.accent, 0.18) : "transparent"
                border.color: theme.line

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.left: parent.left
                    anchors.leftMargin: 10
                    text: source.modelData.label
                    color: theme.textStrong
                    font.pixelSize: 12
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.right: parent.right
                    anchors.rightMargin: 10
                    text: "Import"
                    color: theme.accent
                    font.pixelSize: 12
                    font.bold: true
                }
                MouseArea {
                    id: sourceHover
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: root.showResult(importer.importSource(source.modelData.id))
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 30
            radius: Math.min(root.cornerRadius, 6)
            color: csvHover.containsMouse ? Qt.alpha(theme.accent, 0.18) : "transparent"
            border.color: theme.line

            Text {
                anchors.centerIn: parent
                text: "Import a passwords CSV file"
                color: theme.textStrong
                font.pixelSize: 12
            }
            MouseArea {
                id: csvHover
                anchors.fill: parent
                hoverEnabled: true
                onClicked: csvDialog.open()
            }
        }

        Text {
            Layout.fillWidth: true
            visible: root.importResult.length > 0
            text: root.importResult
            wrapMode: Text.WordWrap
            color: root.importFailed ? theme.red : theme.green
            font.pixelSize: 11
        }
    }
}
