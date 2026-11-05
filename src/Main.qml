import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine

ApplicationWindow {
    id: appWindow
    visible: true
    width: 1200
    height: 800
    title: tabsModel.count > 0 ? tabsModel.get(currentTabIndex).title + " — nodaysidle" : "nodaysidle"
    color: theme.background

    // Tabs Model
    ListModel {
        id: tabsModel
    }

    property int currentTabIndex: 0
    property bool bookmarksBarVisible: true
    property bool assistantVisible: false

    Component.onCompleted: {
        newTab(backend.homeUrl);
    }

    function newTab(url) {
        if (!url || url.length === 0)
            url = backend.homeUrl;
        tabsModel.append({
            title: "New Tab",
            url: url,
            loading: false,
            canGoBack: false,
            canGoForward: false,
            icon: ""
        });
        currentTabIndex = tabsModel.count - 1;
    }

    function closeTab(index) {
        if (index < 0 || index >= tabsModel.count)
            return;
        tabsModel.remove(index);
        if (tabsModel.count === 0) {
            newTab(backend.homeUrl);
        } else if (currentTabIndex >= tabsModel.count) {
            currentTabIndex = tabsModel.count - 1;
        }
    }

    function currentView() {
        return webViewRepeater.itemAt(currentTabIndex);
    }

    // Keyboard Shortcuts
    Shortcut { sequence: "Ctrl+T"; onActivated: newTab(backend.homeUrl) }
    Shortcut { sequence: "Ctrl+W"; onActivated: closeTab(currentTabIndex) }
    Shortcut { sequence: "Ctrl+R"; onActivated: { var v = currentView(); if (v) v.reload(); } }
    Shortcut { sequence: "Ctrl+L"; onActivated: { urlInput.selectAll(); urlInput.forceActiveFocus(); } }
    Shortcut { sequence: "Alt+D"; onActivated: { urlInput.selectAll(); urlInput.forceActiveFocus(); } }
    Shortcut { sequence: "Ctrl+D"; onActivated: {
        var v = currentView();
        if (v) backend.toggleBookmark(v.title, v.url.toString());
    }}
    Shortcut { sequence: "Ctrl+B"; onActivated: bookmarksBarVisible = !bookmarksBarVisible }
    Shortcut { sequence: "Ctrl+Shift+A"; onActivated: assistantVisible = !assistantVisible }
    Shortcut { sequence: "Alt+Left"; onActivated: { var v = currentView(); if (v && v.canGoBack) v.goBack(); } }
    Shortcut { sequence: "Alt+Right"; onActivated: { var v = currentView(); if (v && v.canGoForward) v.goForward(); } }
    Shortcut { sequence: "Escape"; onActivated: { if (assistantVisible) assistantVisible = false; } }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Top Chrome: Tab Strip
        Rectangle {
            Layout.fillWidth: true
            height: 38
            color: theme.chrome

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 8
                anchors.rightMargin: 8
                spacing: 4

                // Tab Pills List
                ListView {
                    id: tabsList
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    orientation: ListView.Horizontal
                    model: tabsModel
                    spacing: 4
                    clip: true

                    delegate: Rectangle {
                        id: tabPill
                        width: Math.max(120, Math.min(220, (tabsList.width - 40) / Math.max(1, tabsModel.count)))
                        height: 30
                        anchors.verticalCenter: parent.verticalCenter
                        radius: 6
                        color: index === currentTabIndex ? theme.surface : (tabHover.hovered ? Qt.alpha(theme.surface, 0.5) : "transparent")
                        border.color: index === currentTabIndex ? theme.line : "transparent"

                        // Active accent indicator bar at bottom
                        Rectangle {
                            anchors.bottom: parent.bottom
                            anchors.left: parent.left
                            anchors.right: parent.right
                            height: 2
                            radius: 1
                            color: theme.accent
                            visible: index === currentTabIndex
                        }

                        MouseArea {
                            id: tabHover
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                            onClicked: function(mouse) {
                                if (mouse.button === Qt.MiddleButton) {
                                    closeTab(index);
                                } else {
                                    currentTabIndex = index;
                                }
                            }
                        }

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            anchors.rightMargin: 8
                            spacing: 6

                            // Loading dot or Globe
                            Text {
                                text: model.loading ? "●" : "🌐"
                                font.pixelSize: 11
                                color: model.loading ? theme.accent : theme.muted
                            }

                            Text {
                                Layout.fillWidth: true
                                text: model.title && model.title.length > 0 ? model.title : "Loading..."
                                color: index === currentTabIndex ? theme.textStrong : theme.text
                                font.pixelSize: 12
                                elide: Text.ElideRight
                            }

                            // Close Button
                            Rectangle {
                                width: 18
                                height: 18
                                radius: 9
                                color: closeHover.hovered ? Qt.alpha(theme.text, 0.2) : "transparent"
                                visible: tabHover.hovered || index === currentTabIndex

                                Text {
                                    anchors.centerIn: parent
                                    text: "×"
                                    font.pixelSize: 14
                                    color: theme.muted
                                }

                                MouseArea {
                                    id: closeHover
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    onClicked: closeTab(index)
                                }
                            }
                        }
                    }
                }

                // New Tab Button (+)
                Rectangle {
                    width: 28
                    height: 28
                    radius: 6
                    color: newTabHover.hovered ? theme.surface : "transparent"

                    Text {
                        anchors.centerIn: parent
                        text: "+"
                        font.pixelSize: 16
                        color: theme.textStrong
                    }

                    MouseArea {
                        id: newTabHover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: newTab(backend.homeUrl)
                    }
                }
            }
        }

        // Toolbar: Navigation buttons & URL Address Bar
        Rectangle {
            Layout.fillWidth: true
            height: 42
            color: theme.chrome

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 10
                anchors.rightMargin: 10
                spacing: 8

                // Back
                Rectangle {
                    width: 28
                    height: 28
                    radius: 6
                    color: backHover.hovered ? theme.surface : "transparent"
                    opacity: (currentView() && currentView().canGoBack) ? 1.0 : 0.4

                    Text {
                        anchors.centerIn: parent
                        text: "‹"
                        font.pixelSize: 18
                        font.bold: true
                        color: theme.textStrong
                    }

                    MouseArea {
                        id: backHover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: { var v = currentView(); if (v && v.canGoBack) v.goBack(); }
                    }
                }

                // Forward
                Rectangle {
                    width: 28
                    height: 28
                    radius: 6
                    color: fwdHover.hovered ? theme.surface : "transparent"
                    opacity: (currentView() && currentView().canGoForward) ? 1.0 : 0.4

                    Text {
                        anchors.centerIn: parent
                        text: "›"
                        font.pixelSize: 18
                        font.bold: true
                        color: theme.textStrong
                    }

                    MouseArea {
                        id: fwdHover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: { var v = currentView(); if (v && v.canGoForward) v.goForward(); }
                    }
                }

                // Reload
                Rectangle {
                    width: 28
                    height: 28
                    radius: 6
                    color: reloadHover.hovered ? theme.surface : "transparent"

                    Text {
                        anchors.centerIn: parent
                        text: (currentView() && currentView().loading) ? "✕" : "⟳"
                        font.pixelSize: 14
                        color: theme.textStrong
                    }

                    MouseArea {
                        id: reloadHover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: {
                            var v = currentView();
                            if (v) {
                                if (v.loading) v.stop();
                                else v.reload();
                            }
                        }
                    }
                }

                // Home
                Rectangle {
                    width: 28
                    height: 28
                    radius: 6
                    color: homeHover.hovered ? theme.surface : "transparent"

                    Text {
                        anchors.centerIn: parent
                        text: "⌂"
                        font.pixelSize: 16
                        color: theme.textStrong
                    }

                    MouseArea {
                        id: homeHover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: {
                            var v = currentView();
                            if (v) v.url = backend.homeUrl;
                        }
                    }
                }

                // URL Address Bar
                Rectangle {
                    Layout.fillWidth: true
                    height: 32
                    radius: 8
                    color: theme.surface
                    border.color: urlInput.activeFocus ? theme.accent : theme.line
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 8
                        spacing: 8

                        // Secure icon
                        Text {
                            text: (currentView() && currentView().url.toString().startsWith("https://")) ? "🔒" : "🌐"
                            font.pixelSize: 12
                            color: (currentView() && currentView().url.toString().startsWith("https://")) ? theme.green : theme.muted
                        }

                        TextField {
                            id: urlInput
                            Layout.fillWidth: true
                            text: (currentView() && !activeFocus) ? (currentView().url.toString() === backend.homeUrl ? "" : currentView().url.toString()) : ""
                            placeholderText: "Search with DuckDuckGo or enter address..."
                            placeholderTextColor: theme.muted
                            color: theme.textStrong
                            selectionColor: theme.accent
                            selectedTextColor: theme.accentForeground
                            font.pixelSize: 13
                            selectByMouse: true
                            clip: true
                            background: null

                            onAccepted: {
                                var target = backend.normalizeUrl(text);
                                var v = currentView();
                                if (v) v.url = target;
                                focus = false;
                            }
                        }

                        // Star Bookmark Button
                        Rectangle {
                            width: 24
                            height: 24
                            radius: 4
                            color: starHover.hovered ? Qt.alpha(theme.text, 0.1) : "transparent"

                            Text {
                                anchors.centerIn: parent
                                text: "★"
                                font.pixelSize: 15
                                color: (currentView() && backend.isBookmarked(currentView().url.toString())) ? theme.accent : theme.muted
                            }

                            MouseArea {
                                id: starHover
                                anchors.fill: parent
                                hoverEnabled: true
                                onClicked: {
                                    var v = currentView();
                                    if (v) backend.toggleBookmark(v.title, v.url.toString());
                                }
                            }
                        }
                    }
                }

                // AI Assistant Toggle Button (💬)
                Rectangle {
                    width: 32
                    height: 28
                    radius: 6
                    color: assistantVisible ? Qt.alpha(theme.accent, 0.25) : (aiHover.hovered ? theme.surface : "transparent")
                    border.color: assistantVisible ? theme.accent : "transparent"

                    Text {
                        anchors.centerIn: parent
                        text: "💬"
                        font.pixelSize: 14
                    }

                    MouseArea {
                        id: aiHover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: assistantVisible = !assistantVisible
                    }
                }
            }
        }

        // Bookmarks Bar (Toggleable with Ctrl+B)
        Rectangle {
            Layout.fillWidth: true
            height: bookmarksBarVisible ? 28 : 0
            visible: bookmarksBarVisible
            color: theme.chrome

            ListView {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                orientation: ListView.Horizontal
                spacing: 8
                model: backend.bookmarks
                clip: true

                delegate: Rectangle {
                    height: 22
                    width: bmText.implicitWidth + 16
                    anchors.verticalCenter: parent.verticalCenter
                    radius: 4
                    color: bmHover.hovered ? theme.surface : "transparent"
                    border.color: bmHover.hovered ? theme.line : "transparent"

                    Text {
                        id: bmText
                        anchors.centerIn: parent
                        text: modelData.title
                        color: bmHover.hovered ? theme.textStrong : theme.text
                        font.pixelSize: 12
                    }

                    MouseArea {
                        id: bmHover
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        onClicked: function(mouse) {
                            if (mouse.button === Qt.LeftButton) {
                                var v = currentView();
                                if (v) v.url = modelData.url;
                            } else if (mouse.button === Qt.RightButton) {
                                bmMenu.popup();
                            }
                        }

                        Menu {
                            id: bmMenu
                            MenuItem {
                                text: "Open in New Tab"
                                onTriggered: newTab(modelData.url)
                            }
                            MenuItem {
                                text: "Delete Bookmark"
                                onTriggered: backend.removeBookmark(modelData.url)
                            }
                        }
                    }
                }
            }
        }

        // Main Content Area: WebEngine Views & Side Assistant Panel
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            RowLayout {
                anchors.fill: parent
                spacing: 0

                // WebEngine Views Container
                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    Repeater {
                        id: webViewRepeater
                        model: tabsModel

                        WebEngineView {
                            id: webView
                            anchors.fill: parent
                            visible: index === currentTabIndex
                            url: model.url

                            onUrlChanged: {
                                if (index < tabsModel.count)
                                    tabsModel.setProperty(index, "url", url.toString());
                            }

                            onTitleChanged: {
                                if (index < tabsModel.count)
                                    tabsModel.setProperty(index, "title", title);
                            }

                            onLoadingChanged: function(loadRequest) {
                                if (index < tabsModel.count) {
                                    tabsModel.setProperty(index, "loading", loadRequest.status === WebEngineView.LoadStartedStatus);
                                    tabsModel.setProperty(index, "canGoBack", canGoBack);
                                    tabsModel.setProperty(index, "canGoForward", canGoForward);
                                }
                            }

                            onNewWindowRequested: function(request) {
                                newTab(request.requestedUrl.toString());
                            }

                            // Custom Right-Click Context Menu for Native Wayland Link Handling
                            onContextMenuRequested: function(request) {
                                request.accepted = true;
                                contextMenu.linkUrl = request.linkUrl.toString();
                                contextMenu.popup();
                            }

                            Menu {
                                id: contextMenu
                                property string linkUrl: ""

                                MenuItem {
                                    text: "Open Link in New Tab"
                                    visible: contextMenu.linkUrl.length > 0
                                    onTriggered: newTab(contextMenu.linkUrl)
                                }
                                MenuItem {
                                    text: "Copy Link Address"
                                    visible: contextMenu.linkUrl.length > 0
                                    onTriggered: {
                                        // Copy to clipboard
                                    }
                                }
                                MenuSeparator { visible: contextMenu.linkUrl.length > 0 }
                                MenuItem {
                                    text: "Back"
                                    enabled: webView.canGoBack
                                    onTriggered: webView.goBack()
                                }
                                MenuItem {
                                    text: "Forward"
                                    enabled: webView.canGoForward
                                    onTriggered: webView.goForward()
                                }
                                MenuItem {
                                    text: "Reload"
                                    onTriggered: webView.reload()
                                }
                            }
                        }
                    }
                }

                // AI Assistant Side Panel (Claude Haiku 5.5 + TypeSafe Jev)
                Rectangle {
                    id: assistantPanel
                    Layout.preferredWidth: assistantVisible ? 360 : 0
                    Layout.fillHeight: true
                    visible: assistantVisible
                    color: theme.surface
                    border.color: theme.line
                    border.width: 1

                    Behavior on Layout.preferredWidth {
                        NumberAnimation { duration: 150; easing.type: Easing.OutQuad }
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 12

                        // Assistant Header
                        RowLayout {
                            Layout.fillWidth: true

                            Text {
                                text: "Ask about this page"
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
                                    onClicked: assistantVisible = false
                                }
                            }
                        }

                        // Status Badge
                        Rectangle {
                            Layout.fillWidth: true
                            height: 24
                            radius: 4
                            color: Qt.alpha(theme.chrome, 0.7)

                            Text {
                                anchors.centerIn: parent
                                text: "⚡ Claude Haiku 5.5 + TypeSafe Jev • Private"
                                font.pixelSize: 11
                                color: theme.text
                            }
                        }

                        // Action Buttons
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            // Summarize Button
                            Rectangle {
                                Layout.fillWidth: true
                                height: 32
                                radius: 6
                                color: sumHover.hovered ? theme.chrome : Qt.alpha(theme.chrome, 0.6)
                                border.color: theme.line

                                Text {
                                    anchors.centerIn: parent
                                    text: "✨ Summarize"
                                    font.pixelSize: 12
                                    font.bold: true
                                    color: theme.textStrong
                                }

                                MouseArea {
                                    id: sumHover
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    onClicked: {
                                        var v = currentView();
                                        if (v) {
                                            aiOutput.text += "\n✨ Summarizing page...\n\n";
                                            v.runJavaScript("document.body.innerText", function(res) {
                                                backend.askAssistant(v.title, v.url.toString(), res, "Summarize this page in a few short paragraphs and key points.");
                                            });
                                        }
                                    }
                                }
                            }

                            // Scam Check Button
                            Rectangle {
                                Layout.fillWidth: true
                                height: 32
                                radius: 6
                                color: scamHover.hovered ? theme.chrome : Qt.alpha(theme.chrome, 0.6)
                                border.color: theme.line

                                Text {
                                    anchors.centerIn: parent
                                    text: "🛡️ Scam check"
                                    font.pixelSize: 12
                                    font.bold: true
                                    color: theme.textStrong
                                }

                                MouseArea {
                                    id: scamHover
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    onClicked: {
                                        var v = currentView();
                                        if (v) {
                                            aiOutput.text += "\n🛡️ Checking security with TypeSafe Jev...\n";
                                            v.runJavaScript("document.body.innerText", function(res) {
                                                backend.checkScam(v.title, v.url.toString(), res);
                                            });
                                        }
                                    }
                                }
                            }
                        }

                        // Assistant Chat / Output Log
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            radius: 8
                            color: theme.chrome
                            border.color: theme.line
                            clip: true

                            ScrollView {
                                anchors.fill: parent
                                anchors.margins: 8

                                TextArea {
                                    id: aiOutput
                                    readOnly: true
                                    wrapMode: TextArea.Wrap
                                    color: theme.textStrong
                                    font.pixelSize: 12
                                    text: "Ready to assist.\n"
                                }
                            }
                        }

                        // Question Input
                        Rectangle {
                            Layout.fillWidth: true
                            height: 36
                            radius: 6
                            color: theme.chrome
                            border.color: theme.line

                            TextField {
                                id: promptInput
                                anchors.fill: parent
                                anchors.margins: 4
                                color: theme.textStrong
                                placeholderText: "Ask Claude about this page... (Enter)"
                                placeholderTextColor: theme.muted
                                font.pixelSize: 12
                                selectByMouse: true
                                background: null
                                onAccepted: {
                                    var q = text.trim();
                                    if (q.length > 0) {
                                        aiOutput.text += "\n💬 " + q + "\n\n";
                                        text = "";
                                        var v = currentView();
                                        if (v) {
                                            v.runJavaScript("document.body.innerText", function(res) {
                                                backend.askAssistant(v.title, v.url.toString(), res, q);
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Connect Backend Signals to UI
    Connections {
        target: backend

        function onAssistantChunk(chunk) {
            aiOutput.text += chunk;
        }

        function onAssistantDone() {
            aiOutput.text += "\n";
        }

        function onAssistantError(err) {
            aiOutput.text += "\n❌ " + err + "\n";
        }

        function onScamResult(verdict, imp, cred, press, host) {
            aiOutput.text += "\n🛡️ Scam Verdict: " + verdict + "\n";
            aiOutput.text += "  • Impersonation risk: " + imp + "%\n";
            aiOutput.text += "  • Credential harvest: " + cred + "%\n";
            aiOutput.text += "  • Pressure / urgency: " + press + "%\n";
            aiOutput.text += "  (Jev estimate for " + host + ")\n\n";
        }
    }
}
