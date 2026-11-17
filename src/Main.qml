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
    // The window close button must save the tabs too, not only Ctrl+Q.
    onClosing: persistSession()

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
    // Injected before the start page's own scripts so it never flashes the default dark theme.
    readonly property var homeThemeScript: [{
        name: "home-theme",
        injectionPoint: WebEngineScript.DocumentCreation,
        worldId: WebEngineScript.MainWorld,
        runsOnSubFrames: false,
        sourceCode: "window.__ndTheme = {" +
            "'--bg':'" + theme.background + "'," +
            "'--surface':'" + theme.surface + "'," +
            "'--line':'" + theme.line + "'," +
            "'--text':'" + theme.text + "'," +
            "'--strong':'" + theme.textStrong + "'," +
            "'--muted':'" + theme.mutedText + "'," +
            "'--accent':'" + theme.accent + "'," +
            "'--radius':'" + Math.max(theme.radius, 4) + "px'};"
    }]
    // Reactive handle on the visible tab's view; toolbar bindings depend on it.
    property var activeView: null
    onCurrentTabIndexChanged: { refreshActiveView(); sessionSaveTimer.restart(); }
    property bool bookmarksBarVisible: true
    property bool scamVisible: false
    property bool scamBusy: false
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
    // A permission request belongs to the tab that asked, so the bar only shows the
    // current tab's; webView.permissionRequest holds each one.
    readonly property var currentPermission: activeView ? activeView.permissionRequest : null
    onCurrentPermissionChanged: if (currentPermission) permAllow.forceActiveFocus()
    // Set when QtWebEngine rejected a certificate; the bar below says which host and why.
    property var certificateProblem: null
    // Downloads wait here until the person answers; see requestDownload().
    property var downloads: []
    // While the saved tabs are being rebuilt, do not write the half-built list back.
    property bool restoringSession: true
    property bool quitPromptVisible: false

    Component.onCompleted: {
        if (backend.initialUrls.length > 0) {
            for (var i = 0; i < backend.initialUrls.length; ++i)
                newTab(backend.initialUrls[i]);
        } else if (backend.sessionTabs.length > 0) {
            var saved = backend.sessionTabs;
            for (var j = 0; j < saved.length; ++j)
                newTab(saved[j]);
            currentTabIndex = Math.max(0, Math.min(saved.length - 1, backend.sessionIndex));
        } else {
            newTab(backend.homeUrl);
        }
        restoringSession = false;
    }

    function persistSession() {
        if (restoringSession)
            return;
        var urls = [];
        for (var i = 0; i < tabsModel.count; ++i)
            urls.push(tabsModel.get(i).url);
        backend.saveSession(urls, currentTabIndex);
    }

    function requestQuit() {
        if (tabsModel.count > 3) {
            quitPromptVisible = true;
            quitCancel.forceActiveFocus();
            return;
        }
        persistSession();
        Qt.quit();
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
        // Ctrl+T and the + button land on the start page; the keyboard should be able to
        // type an address straight away instead of being trapped in the page.
        if (url === backend.homeUrl) {
            urlInput.selectAll();
            urlInput.forceActiveFocus();
        }
    }

    function closeTab(index) {
        if (index < 0 || index >= tabsModel.count)
            return;
        // A tab that goes away must not leave a live permission request behind.
        var closing = webViewRepeater.itemAt(index);
        if (closing && closing.permissionRequest) {
            closing.permissionRequest.deny();
            closing.permissionRequest.reset();
            closing.permissionRequest = null;
        }
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
        sessionSaveTimer.restart();
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
            "s.setProperty('--muted','" + theme.mutedText + "');" +
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

    // The profile reports every download; the shelf below asks before anything is saved.
    Connections {
        target: appProfile
        function onDownloadRequested(download) { requestDownload(download); }
    }

    function refreshHistory() {
        historyItems = backend.searchHistory(historySearch.text, 200);
    }

    function togglePasswords() {
        passwordsVisible = !passwordsVisible;
        if (passwordsVisible) {
            scamVisible = false;
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
            scamVisible = false;
            passwordsVisible = false;
            historySearch.text = "";
            refreshHistory();
            historySearch.forceActiveFocus();
        }
    }

    function runScamCheck() {
        var v = activeView;
        if (scamBusy || !backend.hasJevKey || !v)
            return;
        scamBusy = true;
        scamOutput.text = "Sending this page's text to TypeSafe…\n";
        scamWatchdog.restart();
        v.runJavaScript("document.body.innerText", function(res) {
            backend.checkScam(v.title, v.url.toString(), res);
        });
    }

    function toggleCurrentBookmark() {
        var v = activeView;
        if (v)
            backend.toggleBookmark(v.title, v.url.toString());
    }

    function reloadCrashedTab() {
        var v = activeView;
        if (!v)
            return;
        v.crashed = false;
        v.reload();
    }

    function formatBytes(bytes) {
        if (!bytes || bytes <= 0)
            return "unknown size";
        if (bytes < 1024)
            return bytes + " B";
        if (bytes < 1024 * 1024)
            return (bytes / 1024).toFixed(1) + " KB";
        if (bytes < 1024 * 1024 * 1024)
            return (bytes / (1024 * 1024)).toFixed(1) + " MB";
        return (bytes / (1024 * 1024 * 1024)).toFixed(2) + " GB";
    }

    // Nothing is written to disk until Save: a page cannot drop files on its own.
    function requestDownload(download) {
        downloads = downloads.concat([{ req: download, origin: download.url.toString() }]);
    }

    function acceptDownload(item) {
        item.req.downloadDirectory = backend.downloadDir();
        item.req.accept();
    }

    function dismissDownload(item) {
        downloads = downloads.filter(function (d) { return d !== item; });
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
        var permission = currentPermission;
        if (!permission || !activeView)
            return;
        if (allow) {
            permission.grant();
        } else {
            permission.deny();
            permission.reset();
        }
        activeView.permissionRequest = null;
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
            // A popup closed by the window manager must be destroyed, not just hidden.
            onClosing: destroy()

            property alias webView: popupView

            // Read-only address line, so a popup cannot hide which site it shows.
            Rectangle {
                id: popupBar
                anchors { top: parent.top; left: parent.left; right: parent.right }
                height: 28
                color: theme.chrome
                Text {
                    anchors { fill: parent; leftMargin: 10; rightMargin: 10 }
                    verticalAlignment: Text.AlignVCenter
                    text: popupView.url.toString()
                    color: theme.text
                    font.pixelSize: 12
                    elide: Text.ElideRight
                }
            }

            WebEngineView {
                id: popupView
                anchors { top: popupBar.bottom; left: parent.left; right: parent.right; bottom: parent.bottom }
                profile: appProfile
                userScripts.collection: pageScripts
                // Popup blocker: windows open only from a click or key press.
                settings.javascriptCanOpenWindows: false
                settings.localContentCanAccessFileUrls: false
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
    Shortcut { sequence: "Ctrl+Shift+A"; onActivated: { scamVisible = !scamVisible; if (scamVisible) { historyVisible = false; passwordsVisible = false; } } }
    Shortcut { sequence: "Alt+Left"; onActivated: { var v = currentView(); if (v && v.canGoBack) v.goBack(); } }
    Shortcut { sequence: "Alt+Right"; onActivated: { var v = currentView(); if (v && v.canGoForward) v.goForward(); } }
    Shortcut { sequence: "Ctrl+Tab"; onActivated: cycleTab(1) }
    Shortcut { sequence: "Ctrl+Shift+Tab"; onActivated: cycleTab(-1) }
    Shortcut { sequence: "Ctrl+Q"; onActivated: requestQuit() }
    Shortcut { sequence: "Escape"; enabled: pageFullScreen; onActivated: { var v = currentView(); if (v) v.fullScreenCancelled(); } }
    Shortcut { sequence: "Escape"; enabled: findVisible && !pageFullScreen; onActivated: closeFind() }
    Shortcut { sequence: "Escape"; enabled: passwordsVisible && !pageFullScreen && !findVisible; onActivated: passwordsVisible = false }
    Shortcut { sequence: "Escape"; enabled: historyVisible && !pageFullScreen && !findVisible; onActivated: historyVisible = false }
    Shortcut { sequence: "Escape"; enabled: scamVisible && !pageFullScreen && !findVisible; onActivated: scamVisible = false }

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
                    Accessible.role: Accessible.PageTabList
                    Accessible.name: "Open tabs"

                    delegate: Rectangle {
                        id: tabPill
                        width: Math.max(120, Math.min(220, (tabsList.width - 40) / Math.max(1, tabsModel.count)))
                        height: 30
                        anchors.verticalCenter: parent.verticalCenter
                        radius: appWindow.cornerRadius
                        color: index === currentTabIndex ? theme.surface : (pillHover.hovered ? Qt.alpha(theme.surface, 0.5) : "transparent")
                        border.color: activeFocus ? theme.accent : (index === currentTabIndex ? theme.line : "transparent")

                        activeFocusOnTab: true
                        Accessible.role: Accessible.PageTab
                        Accessible.name: model.title && model.title.length > 0 ? model.title : "New tab"
                        Accessible.onPressAction: currentTabIndex = index
                        Keys.onPressed: function(event) {
                            if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) {
                                currentTabIndex = index;
                                event.accepted = true;
                            } else if (event.key === Qt.Key_Delete) {
                                closeTab(index);
                                event.accepted = true;
                            }
                        }

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
                                    tabPill.forceActiveFocus();
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
                                color: model.loading ? theme.accent : theme.mutedText
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
                            IconButton {
                                icon: "close"
                                buttonSize: 24
                                cornerRadius: 12
                                accessibleName: "Close tab"
                                visible: pillHover.hovered || index === currentTabIndex || activeFocus
                                onClicked: closeTab(index)
                            }
                        }
                    }
                }

                // New Tab Button (+)
                IconButton {
                    icon: "plus"
                    accessibleName: "New tab"
                    cornerRadius: appWindow.cornerRadius
                    onClicked: newTab(backend.homeUrl)
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
                    accessibleName: "Back"
                    cornerRadius: appWindow.cornerRadius
                    opacity: (activeView && activeView.canGoBack) ? 1.0 : 0.4
                    onClicked: { var v = currentView(); if (v && v.canGoBack) v.goBack(); }
                }

                IconButton {
                    icon: "forward"
                    accessibleName: "Forward"
                    cornerRadius: appWindow.cornerRadius
                    opacity: (activeView && activeView.canGoForward) ? 1.0 : 0.4
                    onClicked: { var v = currentView(); if (v && v.canGoForward) v.goForward(); }
                }

                IconButton {
                    icon: (activeView && activeView.loading) ? "stop" : "reload"
                    accessibleName: (activeView && activeView.loading) ? "Stop loading" : "Reload"
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
                    accessibleName: "Home"
                    cornerRadius: appWindow.cornerRadius
                    onClicked: { var v = currentView(); if (v) v.url = backend.homeUrl; }
                }

                // URL Address Bar
                Rectangle {
                    Layout.fillWidth: true
                    height: 32
                    radius: appWindow.cornerRadius
                    color: theme.surface
                    border.color: urlInput.activeFocus ? theme.accent : theme.inputBorder
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
                            border.color: (activeView && activeView.url.toString().startsWith("https://")) ? theme.green : theme.mutedText
                            border.width: 1

                            Rectangle {
                                anchors.centerIn: parent
                                width: 6
                                height: 6
                                radius: 1
                                color: (activeView && activeView.url.toString().startsWith("https://")) ? theme.green : theme.mutedText
                            }
                        }

                        TextField {
                            id: urlInput
                            Accessible.name: "Address and search bar"
                            Layout.fillWidth: true
                            placeholderText: "Search with DuckDuckGo or enter address..."
                            placeholderTextColor: theme.mutedText
                            color: theme.textStrong
                            selectionColor: theme.accent
                            selectedTextColor: theme.accentForeground
                            font.pixelSize: 13
                            selectByMouse: true
                            clip: true
                            background: null

                            onActiveFocusChanged: if (!activeFocus) { suggestions = []; syncUrlField(); }
                            Keys.onEscapePressed: {
                                suggestions = [];
                                focus = false;
                                syncUrlField();
                                var ve = currentView();
                                if (ve) ve.forceActiveFocus();
                            }
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
                                if (v) {
                                    v.url = target;
                                    // Hand the keyboard back to the page: Space and arrows should scroll.
                                    v.forceActiveFocus();
                                }
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
                                                color: theme.mutedText
                                                font.pixelSize: 11
                                            }
                                        }

                                        MouseArea {
                                            anchors.fill: parent
                                            onClicked: {
                                                var v = currentView();
                                                if (v) {
                                                    v.url = modelData.url;
                                                    v.forceActiveFocus();
                                                }
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
                            border.width: 1
                            border.color: activeFocus ? theme.accent : "transparent"

                            activeFocusOnTab: true
                            Accessible.role: Accessible.Button
                            Accessible.name: (activeView && backend.isBookmarked(activeView.url.toString())) ? "Remove bookmark" : "Bookmark this page"
                            Accessible.onPressAction: toggleCurrentBookmark()
                            Keys.onPressed: function(event) {
                                if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) {
                                    toggleCurrentBookmark();
                                    event.accepted = true;
                                }
                            }

                            Canvas {
                                id: starCanvas
                                anchors.centerIn: parent
                                width: 14
                                height: 14
                                property color starColor: (activeView && backend.bookmarks && backend.isBookmarked(activeView.url.toString())) ? theme.accent : theme.mutedText
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
                                    parent.forceActiveFocus();
                                    toggleCurrentBookmark();
                                }
                            }
                        }
                    }
                }

                IconButton {
                    icon: "key"
                    accessibleName: "Saved passwords"
                    active: passwordsVisible
                    cornerRadius: appWindow.cornerRadius
                    onClicked: togglePasswords()
                }

                // Scam check (TypeSafe) toggles the side panel
                IconButton {
                    icon: "shield"
                    accessibleName: "Scam check with TypeSafe"
                    active: scamVisible
                    cornerRadius: appWindow.cornerRadius
                    onClicked: scamVisible = !scamVisible
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
                    Accessible.role: isFolder ? Accessible.Button : Accessible.Link
                    Accessible.name: (isFolder ? "Bookmark folder " : "Bookmark ") + modelData.title

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

        // Download shelf: one row per download, with the origin, the size, and an explicit Save.
        Column {
            Layout.fillWidth: true
            visible: downloads.length > 0 && !pageFullScreen

            Repeater {
                model: downloads

                Rectangle {
                    id: dlRow
                    required property var modelData
                    width: parent.width
                    height: 34
                    color: theme.surface
                    border.color: theme.line

                    readonly property var req: modelData.req
                    readonly property int state: req.downloadState
                    readonly property bool pending: state === WebEngineDownloadRequest.DownloadRequested
                    readonly property bool running: state === WebEngineDownloadRequest.DownloadInProgress
                    readonly property bool done: state === WebEngineDownloadRequest.DownloadCompleted
                    readonly property bool failed: state === WebEngineDownloadRequest.DownloadInterrupted

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 12
                        anchors.rightMargin: 8
                        spacing: 8

                        Text {
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                            font.pixelSize: 12
                            color: theme.textStrong
                            text: {
                                const name = dlRow.req.downloadFileName;
                                if (dlRow.pending)
                                    return name + "  ·  from " + dlRow.modelData.origin + "  ·  " + appWindow.formatBytes(dlRow.req.totalBytes);
                                if (dlRow.running)
                                    return "Downloading " + name + "  ·  " + appWindow.formatBytes(dlRow.req.receivedBytes) + " of " + appWindow.formatBytes(dlRow.req.totalBytes);
                                if (dlRow.done)
                                    return "Saved " + name + " to Downloads";
                                if (dlRow.failed)
                                    return "Download failed: " + name;
                                return name;
                            }
                        }

                        IconButton {
                            visible: dlRow.pending
                            label: "Save"
                            accessibleName: "Save this file to Downloads"
                            cornerRadius: appWindow.cornerRadius
                            onClicked: acceptDownload(dlRow.modelData)
                        }

                        IconButton {
                            visible: dlRow.running
                            label: "Cancel"
                            accessibleName: "Cancel this download"
                            cornerRadius: appWindow.cornerRadius
                            onClicked: dlRow.req.cancel()
                        }

                        IconButton {
                            visible: dlRow.done
                            label: "Show in folder"
                            accessibleName: "Show the file in the Downloads folder"
                            cornerRadius: appWindow.cornerRadius
                            onClicked: backend.openPath(backend.downloadDir())
                        }

                        IconButton {
                            icon: "close"
                            buttonSize: 24
                            accessibleName: "Dismiss this download"
                            cornerRadius: appWindow.cornerRadius
                            onClicked: dismissDownload(dlRow.modelData)
                        }
                    }
                }
            }
        }

        // Certificate failure: the connection stays rejected, and this says which host and why.
        Rectangle {
            Layout.fillWidth: true
            height: 34
            visible: certificateProblem !== null && !pageFullScreen
            color: theme.surface
            Accessible.role: Accessible.AlertMessage
            Accessible.name: certificateProblem ? "Certificate problem on " + certificateProblem.host : ""

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 8
                spacing: 8

                Text {
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    font.pixelSize: 12
                    color: theme.textStrong
                    text: certificateProblem
                          ? "Not secure: " + certificateProblem.host + " has a certificate problem ("
                            + (certificateProblem.description.length > 0 ? certificateProblem.description : "rejected") + ")."
                          : ""
                }

                IconButton {
                    icon: "close"
                    buttonSize: 24
                    accessibleName: "Dismiss the certificate warning"
                    cornerRadius: appWindow.cornerRadius
                    onClicked: certificateProblem = null
                }
            }
        }

        // Permission prompt for the current tab only
        Rectangle {
            Layout.fillWidth: true
            height: 36
            visible: currentPermission !== null && !pageFullScreen
            color: theme.surface
            Accessible.role: Accessible.AlertMessage
            Accessible.name: currentPermission ? currentPermission.origin.toString().replace(/\/$/, "") + " wants access to " + permissionLabel(currentPermission.permissionType) : ""

            // Escape answers No, from anywhere inside the bar.
            Keys.onEscapePressed: answerPermission(false)

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
                    text: currentPermission ? currentPermission.origin.toString().replace(/\/$/, "") + " wants access to " + permissionLabel(currentPermission.permissionType) : ""
                }

                IconButton {
                    id: permAllow
                    height: 24
                    label: "Allow"
                    accessibleName: "Allow this site the requested access"
                    onClicked: answerPermission(true)
                }

                IconButton {
                    height: 24
                    label: "Deny"
                    accessibleName: "Deny this site the requested access"
                    onClicked: answerPermission(false)
                }
            }
        }

        // Offer to save the login that was just used
        Rectangle {
            Layout.fillWidth: true
            height: 36
            visible: pendingLogin !== null && pendingLogin.ready && pendingLogin.view === activeView && !pageFullScreen
            color: theme.surface
            Accessible.role: Accessible.AlertMessage
            Accessible.name: pendingLogin && pendingLogin.ready ? (pendingLogin.state === "update" ? "Update the saved password?" : "Save this password?") : ""

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

                    IconButton {
                        height: 24
                        label: modelData.label
                        active: modelData.primary
                        accessibleName: modelData.label + " this login"
                        onClicked: answerLogin(modelData.action)
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
                    Accessible.name: "Find in page"
                    Layout.preferredWidth: 260
                    placeholderText: "Find in page"
                    placeholderTextColor: theme.mutedText
                    color: theme.textStrong
                    font.pixelSize: 12
                    selectByMouse: true
                    background: Rectangle {
                        radius: appWindow.cornerRadius
                        color: theme.surface
                        border.color: findInput.activeFocus ? theme.accent : theme.inputBorder
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

                IconButton {
                    icon: "close"
                    accessibleName: "Close find bar"
                    cornerRadius: appWindow.cornerRadius
                    onClicked: closeFind()
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
                            // This tab's own permission request, if the page is asking.
                            property var permissionRequest: null
                            // Set when the renderer died; the overlay below explains it.
                            property bool crashed: false
                            property int crashStatus: 0
                            property int crashExitCode: 0
                            // Hidden tabs freeze so memory and CPU do not grow with tab count,
                            // but a tab that has been making sound keeps running.
                            lifecycleState: visible || recentlyAudible ? WebEngineView.LifecycleState.Active
                                                                      : WebEngineView.LifecycleState.Frozen
                            // Load the first address once; a binding would re-fetch (as GET) every page the tab reaches.
                            Component.onCompleted: url = model.url
                            profile: appProfile
                            userScripts.collection: pageScripts.concat(homeThemeScript)

                            // Popup blocker: windows open only from a click or key press.
                            settings.javascriptCanOpenWindows: false
                            settings.localContentCanAccessFileUrls: false
                            settings.webGLEnabled: true
                            settings.accelerated2dCanvasEnabled: true
                            settings.dnsPrefetchEnabled: true

                            onUrlChanged: {
                                if (index < tabsModel.count)
                                    tabsModel.setProperty(index, "url", url.toString());
                                if (index === currentTabIndex)
                                    syncUrlField();
                                sessionSaveTimer.restart();
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
                                if (loadRequest.status === WebEngineView.LoadStartedStatus) {
                                    certificateProblem = null;
                                    if (pendingLogin && pendingLogin.view === webView)
                                        pendingLogin = Object.assign({}, pendingLogin, { navigated: true });
                                }
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

                            // Keep rejecting a bad certificate, but say so instead of failing silently.
                            onCertificateError: function(error) {
                                error.rejectCertificate();
                                certificateProblem = { host: error.url.toString(), description: error.description };
                            }

                            onRenderProcessTerminated: function(status, exitCode) {
                                webView.crashed = status !== WebEngineView.NormalTerminationStatus;
                                webView.crashStatus = status;
                                webView.crashExitCode = exitCode;
                            }

                            onPermissionRequested: function(permission) {
                                // QtWebEngine has no location service key, so a grant could never produce a position.
                                if (permission.permissionType === WebEnginePermission.PermissionType.Geolocation) {
                                    permission.deny();
                                    permission.reset();
                                    return;
                                }
                                // One live request per tab: a newer one answers the older one No.
                                if (webView.permissionRequest) {
                                    webView.permissionRequest.deny();
                                    webView.permissionRequest.reset();
                                }
                                webView.permissionRequest = permission;
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

                    // A dead renderer used to leave a blank tab with no explanation.
                    Rectangle {
                        anchors.fill: parent
                        visible: activeView !== null && activeView.crashed
                        color: theme.background
                        focus: visible
                        Keys.onReturnPressed: appWindow.reloadCrashedTab()

                        ColumnLayout {
                            anchors.centerIn: parent
                            width: Math.min(420, parent.width - 40)
                            spacing: 12

                            Text {
                                Layout.fillWidth: true
                                horizontalAlignment: Text.AlignHCenter
                                wrapMode: Text.WordWrap
                                color: theme.textStrong
                                font.pixelSize: 14
                                font.bold: true
                                text: "This page stopped"
                            }

                            Text {
                                Layout.fillWidth: true
                                horizontalAlignment: Text.AlignHCenter
                                wrapMode: Text.WordWrap
                                color: theme.text
                                font.pixelSize: 12
                                text: activeView
                                      ? "The renderer behind this tab ended (status " + activeView.crashStatus
                                        + ", exit code " + activeView.crashExitCode + "). Reload to try again."
                                      : ""
                            }

                            IconButton {
                                Layout.alignment: Qt.AlignHCenter
                                label: "Reload"
                                accessibleName: "Reload this page"
                                cornerRadius: appWindow.cornerRadius
                                onClicked: appWindow.reloadCrashedTab()
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
                                Accessible.role: Accessible.Button
                                Accessible.name: "Clear all history"
                                Accessible.onPressAction: { backend.clearHistory(); refreshHistory(); }
                                activeFocusOnTab: true
                                font.pixelSize: 12
                                font.bold: true
                                font.underline: activeFocus
                                color: theme.linkText
                                visible: historyItems.length > 0
                                Keys.onPressed: function(event) {
                                    if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) {
                                        backend.clearHistory();
                                        refreshHistory();
                                        event.accepted = true;
                                    }
                                }
                                MouseArea {
                                    anchors.fill: parent
                                    anchors.margins: -6
                                    onClicked: { backend.clearHistory(); refreshHistory(); }
                                }
                            }

                            IconButton {
                                icon: "close"
                                buttonSize: 24
                                accessibleName: "Close history"
                                cornerRadius: appWindow.cornerRadius
                                onClicked: historyVisible = false
                            }
                        }

                        TextField {
                            id: historySearch
                            Accessible.name: "Search history"
                            Layout.fillWidth: true
                            placeholderText: "Search history"
                            placeholderTextColor: theme.mutedText
                            color: theme.textStrong
                            font.pixelSize: 12
                            selectByMouse: true
                            background: Rectangle {
                                radius: appWindow.cornerRadius
                                color: theme.chrome
                                border.color: historySearch.activeFocus ? theme.accent : theme.inputBorder
                            }
                            onTextChanged: refreshHistory()
                        }

                        Text {
                            Layout.fillWidth: true
                            visible: historyItems.length === 0
                            text: historySearch.text.length > 0 ? "Nothing matches." : "No history yet."
                            color: theme.mutedText
                            font.pixelSize: 12
                        }

                        ListView {
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            model: historyItems
                            spacing: 2
                            Accessible.role: Accessible.List
                            Accessible.name: "History entries"

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
                                        color: theme.mutedText
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

                // Scam check side panel: TypeSafe Jev is the only AI integration left.
                // The badge names the destination, because page text leaves the machine.
                Rectangle {
                    id: scamPanel
                    Layout.preferredWidth: scamVisible ? 360 : 0
                    Layout.fillHeight: true
                    visible: scamVisible
                    color: theme.surface
                    border.color: theme.line
                    border.width: 1
                    Accessible.role: Accessible.Pane
                    Accessible.name: "Scam check"

                    Behavior on Layout.preferredWidth {
                        NumberAnimation { duration: 150; easing.type: Easing.OutQuad }
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 14
                        spacing: 12

                        // Header
                        RowLayout {
                            Layout.fillWidth: true

                            Text {
                                text: "Scam check"
                                font.pixelSize: 15
                                font.bold: true
                                color: theme.textStrong
                                Layout.fillWidth: true
                            }

                            IconButton {
                                icon: "close"
                                buttonSize: 24
                                accessibleName: "Close scam check"
                                cornerRadius: appWindow.cornerRadius
                                onClicked: scamVisible = false
                            }
                        }

                        // What leaves the machine, and where it goes.
                        Rectangle {
                            Layout.fillWidth: true
                            height: 28
                            radius: appWindow.cornerRadius
                            color: Qt.alpha(theme.chrome, 0.7)

                            Text {
                                anchors.centerIn: parent
                                width: parent.width - 16
                                horizontalAlignment: Text.AlignHCenter
                                elide: Text.ElideRight
                                text: backend.hasJevKey
                                      ? "Sends this page's text to TypeSafe (api.typesafe.ai, jev-latest)"
                                      : "TYPESAFE_API_KEY is not set, so the check is off"
                                font.pixelSize: 11
                                color: theme.text
                            }
                        }

                        // Check button
                        Rectangle {
                            Layout.fillWidth: true
                            height: 32
                            visible: backend.hasJevKey
                            radius: appWindow.cornerRadius
                            color: checkHover.containsMouse && !scamBusy ? theme.chrome : Qt.alpha(theme.chrome, 0.6)
                            border.color: theme.line

                            Text {
                                anchors.centerIn: parent
                                text: scamBusy ? "Checking…" : "Check this page"
                                font.pixelSize: 12
                                font.bold: true
                                color: theme.textStrong
                            }

                            MouseArea {
                                id: checkHover
                                anchors.fill: parent
                                hoverEnabled: true
                                onClicked: runScamCheck()
                            }
                        }

                        // Verdict log
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
                                    id: scamOutput
                                    Accessible.name: "Scam check result"
                                    readOnly: true
                                    wrapMode: TextArea.Wrap
                                    color: theme.textStrong
                                    font.pixelSize: 12
                                    text: ""
                                    onTextChanged: cursorPosition = length
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Tabs are written to session.json shortly after they change, not only on quit.
    Timer {
        id: sessionSaveTimer
        interval: 500
        onTriggered: persistSession()
    }

    // A TypeSafe answer that never arrives must not leave the panel stuck on "Checking…".
    Timer {
        id: scamWatchdog
        interval: 30000
        onTriggered: {
            if (scamBusy) {
                scamBusy = false;
                scamOutput.text += "\nError: TypeSafe did not answer within 30 s.\n";
            }
        }
    }

    // Ctrl+Q is easy to hit next to Ctrl+W, so many tabs ask first. Cancel keeps the default.
    FocusScope {
        id: quitOverlay
        anchors.fill: parent
        visible: quitPromptVisible
        focus: quitPromptVisible
        Keys.onEscapePressed: quitPromptVisible = false

        Rectangle {
            anchors.fill: parent
            color: Qt.rgba(0, 0, 0, 0.35)
            MouseArea { anchors.fill: parent }
        }

        Rectangle {
            anchors.centerIn: parent
            width: 340
            height: 138
            radius: appWindow.cornerRadius
            color: theme.surface
            border.color: theme.line
            Accessible.role: Accessible.Dialog
            Accessible.name: "Close " + tabsModel.count + " tabs?"

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 10

                Text {
                    Layout.fillWidth: true
                    text: "Close all " + tabsModel.count + " tabs?"
                    color: theme.textStrong
                    font.pixelSize: 14
                    font.bold: true
                    wrapMode: Text.WordWrap
                }

                Text {
                    Layout.fillWidth: true
                    text: "They reopen next time you start."
                    color: theme.mutedText
                    font.pixelSize: 12
                    wrapMode: Text.WordWrap
                }

                RowLayout {
                    Layout.fillWidth: true
                    Item { Layout.fillWidth: true }
                    IconButton {
                        id: quitCancel
                        label: "Cancel"
                        accessibleName: "Keep the tabs open"
                        cornerRadius: appWindow.cornerRadius
                        onClicked: quitPromptVisible = false
                    }
                    IconButton {
                        label: "Quit"
                        accessibleName: "Close the window"
                        cornerRadius: appWindow.cornerRadius
                        onClicked: { persistSession(); Qt.quit(); }
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

        function onAssistantError(err) {
            scamBusy = false;
            scamWatchdog.stop();
            scamOutput.text += "\nError: " + err + "\n";
            toastText.text = err.split("\n")[0];
            toast.visible = true;
            toastTimer.restart();
        }

        function onScamResult(verdict, imp, cred, press, host) {
            scamBusy = false;
            scamWatchdog.stop();
            scamOutput.text += "\nScam Verdict: " + verdict + "\n";
            scamOutput.text += "  • Impersonation risk: " + imp + "%\n";
            scamOutput.text += "  • Credential harvest: " + cred + "%\n";
            scamOutput.text += "  • Pressure / urgency: " + press + "%\n";
            scamOutput.text += "  (Jev estimate for " + host + ")\n\n";
        }
    }
}
