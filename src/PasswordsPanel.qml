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
    // Deleting a login needs a second click on the same row (F18).
    property int confirmDelete: -1

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

    // Enter on a focused row shows or hides its password, like the hover action.
    function revealCurrent() {
        if (loginsList.currentIndex < 0 || loginsList.currentIndex >= logins.length)
            return;
        const id = logins[loginsList.currentIndex].id;
        const next = Object.assign({}, revealed);
        if (next[id] !== undefined)
            delete next[id];
        else
            next[id] = vault.reveal(id);
        revealed = next;
    }

    Connections {
        target: vault
        function onChanged() { root.refresh(); }
    }

    // The import runs on a worker thread and reports back here, on the GUI thread.
    Connections {
        target: importer
        function onImportFinished(result) { root.showResult(result); }
    }

    FileDialog {
        id: csvDialog
        title: "Import passwords from CSV"
        nameFilters: ["Passwords export (*.csv)", "All files (*)"]
        onAccepted: importer.importPasswordsCsv(selectedFile.toString())
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

            IconButton {
                icon: "close"
                buttonSize: 24
                accessibleName: "Close passwords"
                cornerRadius: root.cornerRadius
                onClicked: root.closeRequested()
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
            color: theme.mutedText
            font.pixelSize: 11
        }

        TextField {
            id: search
            Accessible.name: "Search saved passwords"
            Layout.fillWidth: true
            placeholderText: "Search passwords"
            placeholderTextColor: theme.mutedText
            color: theme.textStrong
            font.pixelSize: 12
            selectByMouse: true
            background: Rectangle {
                radius: root.cornerRadius
                color: theme.chrome
                border.color: search.activeFocus ? theme.accent : theme.inputBorder
            }
            onTextChanged: root.refresh()
        }

        Text {
            Layout.fillWidth: true
            visible: root.logins.length === 0 && vault.available
            text: search.text.length > 0 ? "Nothing matches." : "No saved passwords yet. Sign in to a site and choose Save, or import below."
            wrapMode: Text.WordWrap
            color: theme.mutedText
            font.pixelSize: 12
        }

        ListView {
            id: loginsList
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.logins
            spacing: 2
            currentIndex: -1
            Accessible.role: Accessible.List
            Accessible.name: "Saved logins"
            Keys.onUpPressed: if (currentIndex > 0) currentIndex--
            Keys.onDownPressed: if (currentIndex < root.logins.length - 1) currentIndex++
            Keys.onReturnPressed: root.revealCurrent()
            Keys.onEnterPressed: root.revealCurrent()

            delegate: Rectangle {
                id: row
                required property var modelData
                readonly property bool shown: root.revealed[modelData.id] !== undefined
                width: ListView.view.width
                height: 48
                radius: Math.min(root.cornerRadius, 6)
                color: rowHover.hovered || ListView.isCurrentItem ? Qt.alpha(theme.accent, 0.12) : "transparent"

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
                        color: row.shown ? theme.accent : theme.mutedText
                        font.pixelSize: 11
                    }
                }

                Row {
                    id: actions
                    anchors.right: parent.right
                    anchors.rightMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 10
                    visible: rowHover.hovered || ListView.isCurrentItem

                    Repeater {
                        model: [
                            { label: row.shown ? "Hide" : "Show", act: "toggle" },
                            { label: "Copy", act: "copy" },
                            { label: "Delete", act: "delete" }
                        ]

                        Text {
                            required property var modelData
                            text: modelData.act === "delete" && root.confirmDelete === row.modelData.id ? "Sure?" : modelData.label
                            Accessible.role: Accessible.Button
                            Accessible.name: modelData.label + " the password for " + row.modelData.origin.replace(/^https?:\/\//, "")
                            font.pixelSize: 11
                            font.bold: true
                            color: modelData.act === "delete" ? theme.red : theme.linkText

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
                                    } else if (root.confirmDelete !== id) {
                                        root.confirmDelete = id;
                                    } else {
                                        root.confirmDelete = -1;
                                        vault.remove(id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: theme.line }

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
            color: theme.mutedText
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
                Accessible.role: Accessible.Button
                Accessible.name: "Import passwords from " + source.modelData.label

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
                    text: importer.importing ? "Importing…" : "Import"
                    color: importer.importing ? theme.mutedText : theme.linkText
                    font.pixelSize: 12
                    font.bold: true
                }
                MouseArea {
                    id: sourceHover
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: {
                        if (!importer.importing)
                            importer.importSource(source.modelData.id);
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 30
            radius: Math.min(root.cornerRadius, 6)
            color: csvHover.containsMouse ? Qt.alpha(theme.accent, 0.18) : "transparent"
            border.color: theme.line
            Accessible.role: Accessible.Button
            Accessible.name: "Import a passwords CSV file"

            Text {
                anchors.centerIn: parent
                text: importer.importing ? "Importing…" : "Import a passwords CSV file"
                color: importer.importing ? theme.mutedText : theme.textStrong
                font.pixelSize: 12
            }
            MouseArea {
                id: csvHover
                anchors.fill: parent
                hoverEnabled: true
                onClicked: {
                    if (!importer.importing)
                        csvDialog.open();
                }
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
