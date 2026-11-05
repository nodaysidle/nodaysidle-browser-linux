import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Window
import QtWebEngine

ApplicationWindow {
    id: appWindow
    visible: true
    width: 1200
    height: 800
    title: {
        const t = tabsModel.count > 0 ? tabsModel.get(currentTabIndex).title : "";
        return t.length > 0 && t !== "nodaysidle" ? t + " | nodaysidle" : "nodaysidle";
    }
    color: theme.background

    // Tabs Model
    ListModel {
        id: tabsModel
    }

    readonly property int cornerRadius: Math.min(theme.radius, 10)
    property int currentTabIndex: 0
    readonly property var chromeAppScripts: [{
        name: "chrome-app",
        injectionPoint: WebEngineScript.DocumentCreation,
        worldId: WebEngineScript.MainWorld,
        runsOnSubFrames: true,
        sourceCode: backend.readResource(":/chrome_app.js")
    }]
    // The login agent runs in its own world; the nonce proves its console lines came from it.
    readonly property var loginScripts: [{
        name: "login",
        injectionPoint: WebEngineScript.DocumentReady,
        worldId: WebEngineScript.ApplicationWorld,
        runsOnSubFrames: false,
        sourceCode: "var __ndNonce = '" + vault.nonce + "';\n" + backend.readResource(":/login.js")
    }]
    readonly property var pageScripts: chromeAppScripts.concat(loginScripts)
    // Reactive handle on the visible tab's view; toolbar bindings depend on it.
    property var activeView: null
    onCurrentTabIndexChanged: refreshActiveView()
    property bool bookmarksBarVisible: true
    property bool assistantVisible: false
    property bool assistantBusy: false
    property bool pageFullScreen: false
    property bool findVisible: false
    property bool historyVisible: false
    property bool passwordsVisible: false
    property var pendingLogin: null
    readonly property var bookmarkBarItems: {
        var items = [];
        var folders = {};
        var order = [];
        var all = backend.bookmarks;
        for (var i = 0; i < all.length; ++i) {
            var folder = all[i].folder;
            if (!folder) {
                items.push(all[i]);
            } else {
                if (!folders[folder]) { folders[folder] = []; order.push(folder); }
                folders[folder].push(all[i]);
            }
        }
        for (var j = 0; j < order.length; ++j)
            items.push({ title: order[j], url: "", children: folders[order[j]] });
        return items;
    }
    property var historyItems: []
    property var suggestions: []
    property int suggestIndex: -1
    property string findStatus: ""
    property var closedTabs: []
    property var pendingPermission: null

    Component.onCompleted: {
        if (backend.initialUrls.length === 0)
            newTab(backend.homeUrl);
        for (var i = 0; i < backend.initialUrls.length; ++i)
            newTab(backend.initialUrls[i]);
    }

    Connections {
        target: webViewRepeater
        function onItemAdded() { refreshActiveView(); }
        function onItemRemoved() { Qt.callLater(refreshActiveView); }
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
        const closedUrl = tabsModel.get(index).url;
        if (closedUrl !== backend.homeUrl)
            closedTabs = closedTabs.concat([closedUrl]).slice(-20);
        const wasBefore = index < currentTabIndex;
        tabsModel.remove(index);
        if (tabsModel.count === 0) {
            newTab(backend.homeUrl);
        } else if (currentTabIndex >= tabsModel.count) {
            currentTabIndex = tabsModel.count - 1;
        } else if (wasBefore) {
            currentTabIndex = currentTabIndex - 1;
        } else {
            Qt.callLater(refreshActiveView);
        }
    }

    function currentView() {
        return webViewRepeater.itemAt(currentTabIndex);
    }

    function refreshActiveView() {
        activeView = currentView();
        syncUrlField();
    }

    function syncUrlField() {
        if (urlInput.activeFocus)
            return;
        var v = activeView;
        var u = v ? v.url.toString() : "";
        urlInput.text = (u === backend.homeUrl || u === "about:blank") ? "" : u;
    }

    function applyHomeTheme(view) {
        if (!view || view.url.toString() !== backend.homeUrl)
            return;
        view.runJavaScript("(function(s){" +
            "s.setProperty('--bg','" + theme.background + "');" +
            "s.setProperty('--surface','" + theme.surface + "');" +
            "s.setProperty('--line','" + theme.line + "');" +
            "s.setProperty('--text','" + theme.text + "');" +
            "s.setProperty('--strong','" + theme.textStrong + "');" +
            "s.setProperty('--muted','" + theme.muted + "');" +
            "s.setProperty('--accent','" + theme.accent + "');" +
            "s.setProperty('--radius','" + Math.max(theme.radius, 4) + "px');" +
            "})(document.documentElement.style)");
    }

    Connections {
        target: theme
        function onChanged() {
            for (var i = 0; i < webViewRepeater.count; ++i)
                applyHomeTheme(webViewRepeater.itemAt(i));
        }
    }

    function refreshHistory() {
        historyItems = backend.searchHistory(historySearch.text, 200);
    }

    function togglePasswords() {
        passwordsVisible = !passwordsVisible;
        if (passwordsVisible) {
            assistantVisible = false;
            historyVisible = false;
            passwordsPanel.open();
        }
    }

    function loginMessage(view, message) {
        if (!message.startsWith(vault.nonce))
            return;
        var m;
        try { m = JSON.parse(message.slice(vault.nonce.length)); } catch (e) { return; }
        var pageUrl = view.url.toString();
        var mine = pendingLogin !== null && pendingLogin.view === view;
        if (m.t === "need") {
            var js = vault.fillScript(pageUrl, m.user, !m.pw);
            if (js)
                view.runJavaScript(js, WebEngineScript.ApplicationWorld);
        } else if (m.t === "save") {
            var state = vault.offerState(pageUrl, m.user, m.pass);
            if (state === "new" || state === "update")
                pendingLogin = { view: view, url: pageUrl, user: m.user, pass: m.pass, state: state, ready: false, navigated: false };
            else if (mine)
                pendingLogin = null;
        } else if (mine && m.t === "gone") {
            pendingLogin = Object.assign({}, pendingLogin, { ready: true });
        } else if (mine && m.t === "stuck") {
            pendingLogin = null;
        } else if (mine && m.t === "forms" && (pendingLogin.navigated || pageUrl !== pendingLogin.url)) {
            // A page that still asks for a password after the submit means the sign-in failed.
            pendingLogin = m.pw ? null : Object.assign({}, pendingLogin, { ready: true });
        }
    }

    function answerLogin(action) {
        if (!pendingLogin)
            return;
        if (action === "save") {
            vault.save(pendingLogin.url, pendingLogin.user, pendingLogin.pass);
            backend.notice("Password saved");
        } else if (action === "never") {
            vault.never(pendingLogin.url);
        }
        pendingLogin = null;
    }

    function toggleHistory() {
        historyVisible = !historyVisible;
        if (historyVisible) {
            assistantVisible = false;
            passwordsVisible = false;
            historySearch.text = "";
            refreshHistory();
            historySearch.forceActiveFocus();
        }
    }

    function printPage() {
        var v = activeView;
        if (v) v.printToPdf(backend.pdfPath(v.title));
    }

    function reopenTab() {
        if (closedTabs.length === 0)
            return;
        newTab(closedTabs[closedTabs.length - 1]);
        closedTabs = closedTabs.slice(0, -1);
    }

    function zoomBy(step) {
        var v = activeView;
        if (!v) return;
        if (step === 0)
            v.zoomFactor = 1.0;
        else
            v.zoomFactor = Math.max(0.25, Math.min(5.0, v.zoomFactor + step));
    }

    function runFind(backwards) {
        var v = activeView;
        if (!v) return;
        if (findInput.text.length === 0) {
            v.findText("");
            findStatus = "";
            return;
        }
        v.findText(findInput.text, backwards ? WebEngineView.FindBackward : 0);
    }

    function openFind() {
        findVisible = true;
        findInput.forceActiveFocus();
        findInput.selectAll();
    }

    function closeFind() {
        findVisible = false;
        findStatus = "";
        var v = activeView;
        if (v) { v.findText(""); v.forceActiveFocus(); }
    }

    function permissionLabel(type) {
        switch (type) {
        case WebEnginePermission.PermissionType.Geolocation: return "your location";
        case WebEnginePermission.PermissionType.MediaAudioCapture: return "your microphone";
        case WebEnginePermission.PermissionType.MediaVideoCapture: return "your camera";
        case WebEnginePermission.PermissionType.MediaAudioVideoCapture: return "your camera and microphone";
        case WebEnginePermission.PermissionType.Notifications: return "notifications";
        case WebEnginePermission.PermissionType.ClipboardReadWrite: return "your clipboard";
        case WebEnginePermission.PermissionType.DesktopVideoCapture:
        case WebEnginePermission.PermissionType.DesktopAudioVideoCapture: return "your screen";
        default: return "a browser feature";
        }
    }

    function answerPermission(allow) {
        if (!pendingPermission) return;
        if (allow) {
            pendingPermission.grant();
        } else {
            pendingPermission.deny();
            pendingPermission.reset();
        }
        pendingPermission = null;
    }

    function cycleTab(step) {
        if (tabsModel.count > 1)
            currentTabIndex = (currentTabIndex + step + tabsModel.count) % tabsModel.count;
    }

    Component {
        id: popupComponent
        Window {
            id: popupWin
            width: 580
            height: 680
            visible: true
            color: theme.background
            title: popupView.title && popupView.title.length > 0 ? popupView.title + " | nodaysidle" : "nodaysidle"

            property alias webView: popupView

            WebEngineView {
                id: popupView
                anchors.fill: parent
                profile: appProfile
                userScripts.collection: pageScripts
                settings.javascriptCanOpenWindows: true
                settings.webGLEnabled: true
                settings.accelerated2dCanvasEnabled: true

                onJavaScriptConsoleMessage: function(level, message, line, source) { loginMessage(popupView, message); }
                onNewWindowRequested: function(req) {
                    req.openIn(popupView);
                }
                onWindowCloseRequested: {
                    popupWin.close();
                    popupWin.destroy();
                }
            }
        }
    }

    // Keyboard Shortcuts
    Shortcut { sequence: "Ctrl+T"; onActivated: newTab(backend.homeUrl) }
    Shortcut { sequence: "Ctrl+W"; onActivated: closeTab(currentTabIndex) }
    Shortcut { sequences: ["Ctrl+R", "F5"]; onActivated: { var v = currentView(); if (v) v.reload(); } }
    Shortcut { sequences: ["Ctrl+Shift+R", "Ctrl+F5"]; onActivated: { var v = currentView(); if (v) v.reloadAndBypassCache(); } }
    Shortcut { sequence: "Ctrl+Shift+T"; onActivated: reopenTab() }
    Shortcut { sequence: "Ctrl+F"; onActivated: openFind() }
    Shortcut { sequence: "Ctrl+H"; onActivated: toggleHistory() }
    Shortcut { sequence: "Ctrl+Shift+K"; onActivated: togglePasswords() }
    Shortcut { sequence: "Ctrl+P"; onActivated: printPage() }
    Shortcut { sequences: ["Ctrl+G", "F3"]; enabled: findVisible; onActivated: runFind(false) }
    Shortcut { sequences: ["Ctrl+Shift+G", "Shift+F3"]; enabled: findVisible; onActivated: runFind(true) }
    Shortcut { sequences: ["Ctrl++", "Ctrl+="]; onActivated: zoomBy(0.1) }
    Shortcut { sequence: "Ctrl+-"; onActivated: zoomBy(-0.1) }
    Shortcut { sequence: "Ctrl+0"; onActivated: zoomBy(0) }
    Shortcut { sequence: "F6"; onActivated: { urlInput.selectAll(); urlInput.forceActiveFocus(); } }
    Shortcut { sequence: "F11"; enabled: !pageFullScreen; onActivated: appWindow.visibility = appWindow.visibility === Window.FullScreen ? Window.Windowed : Window.FullScreen }
    Shortcut { sequence: "Ctrl+L"; onActivated: { urlInput.selectAll(); urlInput.forceActiveFocus(); } }
    Shortcut { sequence: "Alt+D"; onActivated: { urlInput.selectAll(); urlInput.forceActiveFocus(); } }
    Shortcut { sequence: "Ctrl+D"; onActivated: {
        var v = currentView();
        if (v) backend.toggleBookmark(v.title, v.url.toString());
    }}
    Shortcut { sequence: "Ctrl+B"; onActivated: bookmarksBarVisible = !bookmarksBarVisible }
    Shortcut { sequence: "Ctrl+Shift+A"; onActivated: { assistantVisible = !assistantVisible; if (assistantVisible) { historyVisible = false; passwordsVisible = false; } } }
    Shortcut { sequence: "Alt+Left"; onActivated: { var v = currentView(); if (v && v.canGoBack) v.goBack(); } }
    Shortcut { sequence: "Alt+Right"; onActivated: { var v = currentView(); if (v && v.canGoForward) v.goForward(); } }
    Shortcut { sequence: "Ctrl+Tab"; onActivated: cycleTab(1) }
    Shortcut { sequence: "Ctrl+Shift+Tab"; onActivated: cycleTab(-1) }
    Shortcut { sequence: "Ctrl+Q"; onActivated: Qt.quit() }
    Shortcut { sequence: "Escape"; enabled: pageFullScreen; onActivated: { var v = currentView(); if (v) v.fullScreenCancelled(); } }
    Shortcut { sequence: "Escape"; enabled: findVisible && !pageFullScreen; onActivated: closeFind() }
    Shortcut { sequence: "Escape"; enabled: passwordsVisible && !pageFullScreen && !findVisible; onActivated: passwordsVisible = false }
    Shortcut { sequence: "Escape"; enabled: historyVisible && !pageFullScreen && !findVisible; onActivated: historyVisible = false }
    Shortcut { sequence: "Escape"; enabled: assistantVisible && !pageFullScreen && !findVisible; onActivated: assistantVisible = false }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Top Chrome: Tab Strip
        Rectangle {
            Layout.fillWidth: true
            height: 38
            visible: !pageFullScreen
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
                        radius: appWindow.cornerRadius
                        color: index === currentTabIndex ? theme.surface : (pillHover.hovered ? Qt.alpha(theme.surface, 0.5) : "transparent")
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

                        HoverHandler { id: pillHover }

                        MouseArea {
                            id: tabHover
                            anchors.fill: parent
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

                            // Site icon; the accent dot shows while loading or when the site has none
                            Image {
                                id: favicon
                                Layout.preferredWidth: 14
                                Layout.preferredHeight: 14
                                sourceSize: Qt.size(28, 28)
                                source: model.icon
                                visible: !model.loading && model.icon !== "" && status === Image.Ready
                            }
                            Rectangle {
                                Layout.preferredWidth: 6
                                Layout.preferredHeight: 6
                                radius: 3
                                color: model.loading ? theme.accent : theme.muted
                                visible: !favicon.visible && (model.loading || index === currentTabIndex)
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
                                color: closeHover.containsMouse ? Qt.alpha(theme.text, 0.2) : "transparent"
                                visible: pillHover.hovered || index === currentTabIndex

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
                    radius: appWindow.cornerRadius
                    color: newTabHover.containsMouse ? theme.surface : "transparent"

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
            visible: !pageFullScreen
            color: theme.chrome

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 10
                anchors.rightMargin: 10
                spacing: 8

                IconButton {
                    icon: "back"
                    cornerRadius: appWindow.cornerRadius
                    opacity: (activeView && activeView.canGoBack) ? 1.0 : 0.4
                    onClicked: { var v = currentView(); if (v && v.canGoBack) v.goBack(); }
                }

                IconButton {
                    icon: "forward"
                    cornerRadius: appWindow.cornerRadius
                    opacity: (activeView && activeView.canGoForward) ? 1.0 : 0.4
                    onClicked: { var v = currentView(); if (v && v.canGoForward) v.goForward(); }
                }

                IconButton {
                    icon: (activeView && activeView.loading) ? "stop" : "reload"
                    cornerRadius: appWindow.cornerRadius
                    onClicked: {
                        var v = currentView();
                        if (v) {
                            if (v.loading) v.stop();
                            else v.reload();
                        }
                    }
                }

                IconButton {
                    icon: "home"
                    cornerRadius: appWindow.cornerRadius
                    onClicked: { var v = currentView(); if (v) v.url = backend.homeUrl; }
                }

                // URL Address Bar
                Rectangle {
                    Layout.fillWidth: true
                    height: 32
                    radius: appWindow.cornerRadius
                    color: theme.surface
                    border.color: urlInput.activeFocus ? theme.accent : theme.line
                    border.width: 1

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 8
                        spacing: 8

                        // Secure badge indicator
                        Rectangle {
                            width: 14
                            height: 14
                            radius: 3
                            color: "transparent"
                            border.color: (activeView && activeView.url.toString().startsWith("https://")) ? theme.green : theme.muted
                            border.width: 1

                            Rectangle {
                                anchors.centerIn: parent
                                width: 6
                                height: 6
                                radius: 1
                                color: (activeView && activeView.url.toString().startsWith("https://")) ? theme.green : theme.muted
                            }
                        }

                        TextField {
                            id: urlInput
                            Layout.fillWidth: true
                            placeholderText: "Search with DuckDuckGo or enter address..."
                            placeholderTextColor: theme.muted
                            color: theme.textStrong
                            selectionColor: theme.accent
                            selectedTextColor: theme.accentForeground
                            font.pixelSize: 13
                            selectByMouse: true
                            clip: true
                            background: null

                            onActiveFocusChanged: if (!activeFocus) { suggestions = []; syncUrlField(); }
                            Keys.onEscapePressed: { suggestions = []; focus = false; syncUrlField(); }
                            Keys.onDownPressed: if (suggestions.length > 0) suggestIndex = Math.min(suggestions.length - 1, suggestIndex + 1)
                            Keys.onUpPressed: if (suggestions.length > 0) suggestIndex = Math.max(-1, suggestIndex - 1)
                            onTextEdited: {
                                suggestions = backend.suggest(text, 6);
                                suggestIndex = -1;
                            }

                            onAccepted: {
                                var target = suggestIndex >= 0 ? suggestions[suggestIndex].url : backend.normalizeUrl(text);
                                suggestions = [];
                                var v = currentView();
                                if (v) v.url = target;
                                focus = false;
                            }

                            Popup {
                                y: urlInput.height + 6
                                width: urlInput.width + 80
                                padding: 4
                                visible: suggestions.length > 0 && urlInput.activeFocus
                                closePolicy: Popup.NoAutoClose
                                focus: false

                                background: Rectangle {
                                    color: theme.surface
                                    border.color: theme.line
                                    radius: appWindow.cornerRadius
                                }

                                contentItem: ListView {
                                    implicitHeight: contentHeight
                                    model: suggestions
                                    interactive: false

                                    delegate: Rectangle {
                                        width: ListView.view.width
                                        height: 38
                                        radius: Math.min(appWindow.cornerRadius, 6)
                                        color: index === suggestIndex ? Qt.alpha(theme.accent, 0.22) : "transparent"

                                        Column {
                                            anchors.verticalCenter: parent.verticalCenter
                                            anchors.left: parent.left
                                            anchors.right: parent.right
                                            anchors.margins: 8

                                            Text {
                                                width: parent.width
                                                text: modelData.title
                                                elide: Text.ElideRight
                                                color: theme.textStrong
                                                font.pixelSize: 12
                                            }
                                            Text {
                                                width: parent.width
                                                text: modelData.url
                                                elide: Text.ElideRight
                                                color: theme.muted
                                                font.pixelSize: 11
                                            }
                                        }

                                        MouseArea {
                                            anchors.fill: parent
                                            onClicked: {
                                                var v = currentView();
                                                if (v) v.url = modelData.url;
                                                suggestions = [];
                                                urlInput.focus = false;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Star Bookmark Button (vector canvas)
                        Rectangle {
                            width: 24
                            height: 24
                            radius: appWindow.cornerRadius
                            color: starHover.containsMouse ? Qt.alpha(theme.text, 0.1) : "transparent"

                            Canvas {
                                id: starCanvas
                                anchors.centerIn: parent
                                width: 14
                                height: 14
                                property color starColor: (activeView && backend.bookmarks && backend.isBookmarked(activeView.url.toString())) ? theme.accent : theme.muted
                                onStarColorChanged: requestPaint()
                                onPaint: {
                                    var ctx = getContext("2d");
                                    ctx.reset();
                                    ctx.fillStyle = starColor;
                                    var rot = Math.PI / 2 * 3;
                                    var cx = 7;
                                    var cy = 7;
                                    var step = Math.PI / 5;
                                    var outer = 6.5;
                                    var inner = 2.8;
                                    ctx.beginPath();
                                    ctx.moveTo(cx, cy - outer);
                                    for (var i = 0; i < 5; i++) {
                                        ctx.lineTo(cx + Math.cos(rot) * outer, cy + Math.sin(rot) * outer);
                                        rot += step;
                                        ctx.lineTo(cx + Math.cos(rot) * inner, cy + Math.sin(rot) * inner);
                                        rot += step;
                                    }
                                    ctx.lineTo(cx, cy - outer);
                                    ctx.closePath();
                                    ctx.fill();
                                }
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

                IconButton {
                    icon: "key"
                    active: passwordsVisible
                    cornerRadius: appWindow.cornerRadius
                    onClicked: togglePasswords()
                }

                // AI Assistant Toggle Button
                Rectangle {
                    width: 32
                    height: 28
                    radius: appWindow.cornerRadius
                    color: assistantVisible ? Qt.alpha(theme.accent, 0.25) : (aiHover.containsMouse ? theme.surface : "transparent")
                    border.color: assistantVisible ? theme.accent : "transparent"

                    Text {
                        anchors.centerIn: parent
                        text: "AI"
                        font.pixelSize: 11
                        font.bold: true
                        color: assistantVisible ? theme.accent : theme.textStrong
                    }

                    MouseArea {
                        id: aiHover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: { assistantVisible = !assistantVisible; if (assistantVisible) { historyVisible = false; passwordsVisible = false; } }
                    }
                }
            }
        }

        // Bookmarks Bar (Toggleable with Ctrl+B)
        Rectangle {
            Layout.fillWidth: true
            height: visible ? 28 : 0
            visible: bookmarksBarVisible && bookmarkBarItems.length > 0 && !pageFullScreen
            color: theme.chrome

            ListView {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                orientation: ListView.Horizontal
                spacing: 8
                model: bookmarkBarItems
                clip: true

                delegate: Rectangle {
                    readonly property bool isFolder: modelData.children !== undefined
                    height: 22
                    width: bmText.implicitWidth + 16
                    anchors.verticalCenter: parent.verticalCenter
                    radius: appWindow.cornerRadius
                    color: bmHover.containsMouse ? theme.surface : "transparent"
                    border.color: bmHover.containsMouse ? theme.line : "transparent"

                    Text {
                        id: bmText
                        anchors.centerIn: parent
                        text: isFolder ? modelData.title + " ▾" : modelData.title
                        color: bmHover.containsMouse ? theme.textStrong : theme.text
                        font.pixelSize: 12
                    }

                    MouseArea {
                        id: bmHover
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        onClicked: function(mouse) {
                            if (isFolder) {
                                if (mouse.button === Qt.LeftButton) folderMenu.popup();
                            } else if (mouse.button === Qt.LeftButton) {
                                var v = currentView();
                                if (v) v.url = modelData.url;
                            } else if (mouse.button === Qt.RightButton) {
                                bmMenu.popup();
                            }
                        }

                        ThemedMenu {
                            id: folderMenu
                            Repeater {
                                model: isFolder ? modelData.children : []
                                ThemedMenuItem {
                                    required property var modelData
                                    text: modelData.title
                                    onTriggered: { var v = currentView(); if (v) v.url = modelData.url; }
                                }
                            }
                        }

                        ThemedMenu {
                            id: bmMenu
                            ThemedMenuItem {
                                text: "Open in New Tab"
                                onTriggered: newTab(modelData.url)
                            }
                            ThemedMenuItem {
                                text: "Delete Bookmark"
                                onTriggered: backend.removeBookmark(modelData.url)
                            }
                        }
                    }
                }
            }
        }

        // Permission prompt
        Rectangle {
            Layout.fillWidth: true
            height: 36
            visible: pendingPermission !== null && !pageFullScreen
            color: theme.surface

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 10

                Text {
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    color: theme.textStrong
                    font.pixelSize: 12
                    text: pendingPermission ? pendingPermission.origin.toString().replace(/\/$/, "") + " wants access to " + permissionLabel(pendingPermission.permissionType) : ""
                }

                Repeater {
                    model: [{ label: "Allow", allow: true }, { label: "Deny", allow: false }]

                    Rectangle {
                        width: 56
                        height: 24
                        radius: appWindow.cornerRadius
                        color: permHover.containsMouse ? Qt.alpha(theme.accent, 0.25) : "transparent"
                        border.color: modelData.allow ? theme.accent : theme.line

                        Text {
                            anchors.centerIn: parent
                            text: modelData.label
                            font.pixelSize: 12
                            font.bold: true
                            color: theme.textStrong
                        }

                        MouseArea {
                            id: permHover
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: answerPermission(modelData.allow)
                        }
                    }
                }
            }
        }

        // Offer to save the login that was just used
        Rectangle {
            Layout.fillWidth: true
            height: 36
            visible: pendingLogin !== null && pendingLogin.ready && pendingLogin.view === activeView && !pageFullScreen
            color: theme.surface

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 10

                Text {
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    color: theme.textStrong
                    font.pixelSize: 12
                    text: pendingLogin
                        ? (pendingLogin.state === "update" ? "Update the saved password for " : "Save the password for ")
                          + (pendingLogin.user.length > 0 ? pendingLogin.user + " on " : "") + pendingLogin.url.replace(/^https?:\/\//, "").split("/")[0] + "?"
                        : ""
                }

                Repeater {
                    model: [
                        { label: pendingLogin && pendingLogin.state === "update" ? "Update" : "Save", action: "save", primary: true },
                        { label: "Never", action: "never", primary: false },
                        { label: "Not now", action: "dismiss", primary: false }
                    ]

                    Rectangle {
                        width: modelData.action === "dismiss" ? 66 : 56
                        height: 24
                        radius: appWindow.cornerRadius
                        color: loginHover.containsMouse ? Qt.alpha(theme.accent, 0.25) : "transparent"
                        border.color: modelData.primary ? theme.accent : theme.line

                        Text {
                            anchors.centerIn: parent
                            text: modelData.label
                            font.pixelSize: 12
                            font.bold: true
                            color: theme.textStrong
                        }

                        MouseArea {
                            id: loginHover
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: answerLogin(modelData.action)
                        }
                    }
                }
            }
        }

        // Find in page (Ctrl+F)
        Rectangle {
            Layout.fillWidth: true
            height: 36
            visible: findVisible && !pageFullScreen
            color: theme.chrome

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 8

                TextField {
                    id: findInput
                    Layout.preferredWidth: 260
                    placeholderText: "Find in page"
                    placeholderTextColor: theme.muted
                    color: theme.textStrong
                    font.pixelSize: 12
                    selectByMouse: true
                    background: Rectangle {
                        radius: appWindow.cornerRadius
                        color: theme.surface
                        border.color: findInput.activeFocus ? theme.accent : theme.line
                    }
                    onTextChanged: runFind(false)
                    onAccepted: runFind(false)
                    Keys.onPressed: function(event) {
                        if (event.key === Qt.Key_Return && (event.modifiers & Qt.ShiftModifier)) {
                            runFind(true);
                            event.accepted = true;
                        }
                    }
                }

                Text {
                    text: findStatus
                    color: theme.text
                    font.pixelSize: 12
                }

                Item { Layout.fillWidth: true }

                Text {
                    text: "×"
                    font.pixelSize: 18
                    color: theme.muted
                    MouseArea {
                        anchors.fill: parent
                        anchors.margins: -6
                        onClicked: closeFind()
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
                            // Load the first address once; a binding would re-fetch (as GET) every page the tab reaches.
                            Component.onCompleted: url = model.url
                            profile: appProfile
                            userScripts.collection: pageScripts

                            settings.javascriptCanOpenWindows: true
                            settings.webGLEnabled: true
                            settings.accelerated2dCanvasEnabled: true
                            settings.dnsPrefetchEnabled: true

                            onUrlChanged: {
                                if (index < tabsModel.count)
                                    tabsModel.setProperty(index, "url", url.toString());
                                if (index === currentTabIndex)
                                    syncUrlField();
                            }

                            onJavaScriptConsoleMessage: function(level, message, line, source) { loginMessage(webView, message); }

                            onIconChanged: {
                                if (index < tabsModel.count)
                                    tabsModel.setProperty(index, "icon", icon.toString());
                            }

                            onTitleChanged: {
                                if (!loading)
                                    backend.recordVisit(url.toString(), title);
                                if (index < tabsModel.count)
                                    tabsModel.setProperty(index, "title", title);
                            }

                            onLoadingChanged: function(loadRequest) {
                                if (loadRequest.status === WebEngineView.LoadStartedStatus && pendingLogin && pendingLogin.view === webView)
                                    pendingLogin = Object.assign({}, pendingLogin, { navigated: true });
                                if (loadRequest.status === WebEngineView.LoadSucceededStatus) {
                                    applyHomeTheme(webView);
                                    backend.recordVisit(url.toString(), title);
                                }
                                if (index < tabsModel.count) {
                                    tabsModel.setProperty(index, "loading", loadRequest.status === WebEngineView.LoadStartedStatus);
                                    tabsModel.setProperty(index, "canGoBack", canGoBack);
                                    tabsModel.setProperty(index, "canGoForward", canGoForward);
                                }
                            }

                            onPdfPrintingFinished: function(filePath, success) {
                                backend.notice(success ? "Saved " + filePath.split("/").pop() + " to Downloads" : "Could not save the PDF");
                            }

                            onFindTextFinished: function(result) {
                                if (index === currentTabIndex)
                                    findStatus = result.numberOfMatches > 0
                                        ? result.activeMatch + " of " + result.numberOfMatches
                                        : "No matches";
                            }

                            onPermissionRequested: function(permission) {
                                // QtWebEngine has no location service key, so a grant could never produce a position.
                                if (permission.permissionType === WebEnginePermission.PermissionType.Geolocation) {
                                    permission.deny();
                                    permission.reset();
                                    return;
                                }
                                if (pendingPermission) {
                                    pendingPermission.deny();
                                    pendingPermission.reset();
                                }
                                pendingPermission = permission;
                            }

                            onFullScreenRequested: function(request) {
                                request.accept();
                                pageFullScreen = request.toggleOn;
                                appWindow.visibility = request.toggleOn ? Window.FullScreen : Window.Windowed;
                            }

                            onNewWindowRequested: function(request) {
                                if (request.destination === WebEngineNewWindowRequest.InNewDialog ||
                                    request.destination === WebEngineNewWindowRequest.InNewWindow) {
                                    var w = popupComponent.createObject(appWindow, {
                                        width: request.requestedGeometry.width > 200 ? request.requestedGeometry.width : 580,
                                        height: request.requestedGeometry.height > 200 ? request.requestedGeometry.height : 680
                                    });
                                    if (w) {
                                        request.openIn(w.webView);
                                        w.show();
                                        return;
                                    }
                                }
                                newTab(request.requestedUrl.toString());
                            }

                            // Custom Right-Click Context Menu for Native Wayland Link Handling
                            onContextMenuRequested: function(request) {
                                request.accepted = true;
                                contextMenu.linkUrl = request.linkUrl.toString();
                                contextMenu.hasSelection = request.selectedText.length > 0;
                                contextMenu.canCut = (request.editFlags & ContextMenuRequest.CanCut) !== 0;
                                contextMenu.canPaste = (request.editFlags & ContextMenuRequest.CanPaste) !== 0;
                                contextMenu.popup();
                            }

                            ThemedMenu {
                                id: contextMenu
                                property string linkUrl: ""
                                property bool hasSelection: false
                                property bool canCut: false
                                property bool canPaste: false

                                ThemedMenuItem {
                                    text: "Cut"
                                    visible: contextMenu.canCut
                                    onTriggered: webView.triggerWebAction(WebEngineView.Cut)
                                }
                                ThemedMenuItem {
                                    text: "Copy"
                                    visible: contextMenu.hasSelection
                                    onTriggered: webView.triggerWebAction(WebEngineView.Copy)
                                }
                                ThemedMenuItem {
                                    text: "Paste"
                                    visible: contextMenu.canPaste
                                    onTriggered: webView.triggerWebAction(WebEngineView.Paste)
                                }

                                ThemedMenuItem {
                                    text: "Open Link in New Tab"
                                    visible: contextMenu.linkUrl.length > 0
                                    onTriggered: newTab(contextMenu.linkUrl)
                                }
                                ThemedMenuItem {
                                    text: "Copy Link Address"
                                    visible: contextMenu.linkUrl.length > 0
                                    onTriggered: backend.copyText(contextMenu.linkUrl)
                                }
                                MenuSeparator {
                                    visible: contextMenu.linkUrl.length > 0
                                    height: visible ? implicitHeight : 0
                                    contentItem: Rectangle { implicitHeight: 1; color: theme.line }
                                }
                                ThemedMenuItem {
                                    text: "Back"
                                    enabled: webView.canGoBack
                                    onTriggered: webView.goBack()
                                }
                                ThemedMenuItem {
                                    text: "Forward"
                                    enabled: webView.canGoForward
                                    onTriggered: webView.goForward()
                                }
                                ThemedMenuItem {
                                    text: "Reload"
                                    onTriggered: webView.reload()
                                }
                            }
                        }
                    }
                }

                // History side panel (Ctrl+H)
                Rectangle {
                    Layout.preferredWidth: 360
                    Layout.fillHeight: true
                    visible: historyVisible && !pageFullScreen
                    color: theme.surface
                    border.color: theme.line

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 10

                        RowLayout {
                            Layout.fillWidth: true

                            Text {
                                text: "History"
                                font.pixelSize: 15
                                font.bold: true
                                color: theme.textStrong
                                Layout.fillWidth: true
                            }

                            Text {
                                text: "Clear all"
                                font.pixelSize: 12
                                font.bold: true
                                color: theme.accent
                                visible: historyItems.length > 0
                                MouseArea {
                                    anchors.fill: parent
                                    anchors.margins: -6
                                    onClicked: { backend.clearHistory(); refreshHistory(); }
                                }
                            }

                            Text {
                                text: "×"
                                font.pixelSize: 18
                                color: theme.muted
                                MouseArea {
                                    anchors.fill: parent
                                    anchors.margins: -4
                                    onClicked: historyVisible = false
                                }
                            }
                        }

                        TextField {
                            id: historySearch
                            Layout.fillWidth: true
                            placeholderText: "Search history"
                            placeholderTextColor: theme.muted
                            color: theme.textStrong
                            font.pixelSize: 12
                            selectByMouse: true
                            background: Rectangle {
                                radius: appWindow.cornerRadius
                                color: theme.chrome
                                border.color: historySearch.activeFocus ? theme.accent : theme.line
                            }
                            onTextChanged: refreshHistory()
                        }

                        Text {
                            Layout.fillWidth: true
                            visible: historyItems.length === 0
                            text: historySearch.text.length > 0 ? "Nothing matches." : "No history yet."
                            color: theme.muted
                            font.pixelSize: 12
                        }

                        ListView {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            model: historyItems
                            spacing: 2

                            delegate: Rectangle {
                                width: ListView.view.width
                                height: 46
                                radius: Math.min(appWindow.cornerRadius, 6)
                                color: histHover.containsMouse ? Qt.alpha(theme.accent, 0.18) : "transparent"

                                Column {
                                    anchors.verticalCenter: parent.verticalCenter
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.margins: 8

                                    Text {
                                        width: parent.width
                                        text: modelData.title
                                        elide: Text.ElideRight
                                        color: theme.textStrong
                                        font.pixelSize: 12
                                    }
                                    Text {
                                        width: parent.width
                                        text: Qt.formatDateTime(new Date(modelData.time * 1000), "MMM d, HH:mm") + "  " + modelData.url
                                        elide: Text.ElideRight
                                        color: theme.muted
                                        font.pixelSize: 11
                                    }
                                }

                                MouseArea {
                                    id: histHover
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    onClicked: {
                                        var v = currentView();
                                        if (v) v.url = modelData.url;
                                    }
                                }
                            }
                        }
                    }
                }

                PasswordsPanel {
                    id: passwordsPanel
                    Layout.preferredWidth: 360
                    Layout.fillHeight: true
                    visible: passwordsVisible && !pageFullScreen
                    cornerRadius: appWindow.cornerRadius
                    onCloseRequested: passwordsVisible = false
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
                                text: "Stop"
                                visible: assistantBusy
                                font.pixelSize: 12
                                font.bold: true
                                color: theme.accent
                                MouseArea {
                                    anchors.fill: parent
                                    anchors.margins: -6
                                    onClicked: backend.cancelAssistant()
                                }
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
                            radius: appWindow.cornerRadius
                            color: Qt.alpha(theme.chrome, 0.7)

                            Text {
                                anchors.centerIn: parent
                                text: "Claude Haiku 5.5 + TypeSafe Jev [Private]"
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
                                radius: appWindow.cornerRadius
                                color: sumHover.containsMouse ? theme.chrome : Qt.alpha(theme.chrome, 0.6)
                                border.color: theme.line

                                Text {
                                    anchors.centerIn: parent
                                    text: "Summarize"
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
                                            aiOutput.text += "\nSummarizing page...\n\n";
                                            v.runJavaScript("document.body.innerText", function(res) {
                                                assistantBusy = true;
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
                                radius: appWindow.cornerRadius
                                color: scamHover.containsMouse ? theme.chrome : Qt.alpha(theme.chrome, 0.6)
                                border.color: theme.line

                                Text {
                                    anchors.centerIn: parent
                                    text: "Scam check"
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
                                            aiOutput.text += "\nChecking security with TypeSafe Jev...\n";
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
                            radius: appWindow.cornerRadius
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
                                    onTextChanged: cursorPosition = length
                                }
                            }
                        }

                        // Question Input
                        Rectangle {
                            Layout.fillWidth: true
                            height: 36
                            radius: appWindow.cornerRadius
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
                                        aiOutput.text += "\nUser: " + q + "\n\n";
                                        text = "";
                                        var v = currentView();
                                        if (v) {
                                            v.runJavaScript("document.body.innerText", function(res) {
                                                assistantBusy = true;
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

    // Transient message such as a finished download
    Rectangle {
        id: toast
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 24
        width: toastText.implicitWidth + 28
        height: 34
        radius: appWindow.cornerRadius
        color: theme.surface
        border.color: theme.line
        visible: false

        Text {
            id: toastText
            anchors.centerIn: parent
            color: theme.textStrong
            font.pixelSize: 12
        }

        Timer {
            id: toastTimer
            interval: 3500
            onTriggered: toast.visible = false
        }
    }

    // Connect Backend Signals to UI
    Connections {
        target: backend

        function onOpenRequested(url) {
            newTab(url);
            appWindow.raise();
            appWindow.requestActivate();
        }

        function onNotice(text) {
            toastText.text = text;
            toast.visible = true;
            toastTimer.restart();
        }

        function onAssistantChunk(chunk) {
            aiOutput.text += chunk;
        }

        function onAssistantDone() {
            assistantBusy = false;
            aiOutput.text += "\n";
        }

        function onAssistantError(err) {
            assistantBusy = false;
            aiOutput.text += "\nError: " + err + "\n";
        }

        function onScamResult(verdict, imp, cred, press, host) {
            aiOutput.text += "\nScam Verdict: " + verdict + "\n";
            aiOutput.text += "  • Impersonation risk: " + imp + "%\n";
            aiOutput.text += "  • Credential harvest: " + cred + "%\n";
            aiOutput.text += "  • Pressure / urgency: " + press + "%\n";
            aiOutput.text += "  (Jev estimate for " + host + ")\n\n";
        }
    }
}
