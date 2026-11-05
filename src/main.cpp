#include <QGuiApplication>
#include <QLocalServer>
#include <QLocalSocket>
#include <unistd.h>
#include <QIcon>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QStandardPaths>
#include <QtWebEngineQuick/qtwebenginequickglobal.h>
#include <QWebEngineProfile>
#include <QWebEngineSettings>
#include <QtWebEngineCore/qtwebenginecoreglobal.h>
#include <QQuickWebEngineProfile>
#include <QtWebEngineQuick/qquickwebenginedownloadrequest.h>

#include "adblock.h"
#include "backend.h"
#include "importer.h"
#include "theme.h"
#include "vault.h"

int main(int argc, char *argv[]) {
    // 1. Force native Wayland platform (Zero XWayland / X11 overhead)
    qputenv("QT_QPA_PLATFORM", "wayland");
    qputenv("QT_WAYLAND_DISABLE_WINDOWDECORATION", "1");
    qputenv("NO_AT_BRIDGE", "1");

    // Disable in-process GPU sandbox trap and let QtWebEngine use native Wayland hardware acceleration
    qputenv("QTWEBENGINE_CHROMIUM_FLAGS",
            "--ignore-gpu-blocklist "
            "--enable-gpu-rasterization "
            "--enable-zero-copy "
            "--no-sandbox "
            + qgetenv("QTWEBENGINE_CHROMIUM_FLAGS"));

    QtWebEngineQuick::initialize();

    QGuiApplication app(argc, argv);
    app.setApplicationName("nodaysidle-browser");
    app.setApplicationDisplayName("nodaysidle");
    app.setApplicationVersion("1.0.0");
    app.setDesktopFileName("com.nodaysidle.Browser");

    QQuickStyle::setStyle("Basic");

    QStringList args;
    for (int i = 1; i < argc; ++i)
        if (argv[i][0] != '-')
            args << QString::fromUtf8(argv[i]);

    Theme theme;
    Backend backend;
    QStringList urls;
    for (const auto &arg : args)
        urls << backend.normalizeUrl(arg);

    // One instance owns the profile; later launches hand their URLs to it and exit.
    const QString serverName = QString("nodaysidle-browser-%1").arg(getuid());
    QLocalSocket client;
    client.connectToServer(serverName);
    if (client.waitForConnected(300)) {
        client.write(urls.join('\n').toUtf8() + "\n");
        client.waitForBytesWritten(1000);
        return 0;
    }
    QLocalServer::removeServer(serverName);
    QLocalServer server;
    server.setSocketOptions(QLocalServer::UserAccessOption);
    server.listen(serverName);
    QObject::connect(&server, &QLocalServer::newConnection, &backend, [&server, &backend] {
        auto *socket = server.nextPendingConnection();
        QObject::connect(socket, &QLocalSocket::disconnected, socket, &QObject::deleteLater);
        QObject::connect(socket, &QLocalSocket::readyRead, &backend, [socket, &backend] {
            const auto lines = QString::fromUtf8(socket->readAll()).split('\n', Qt::SkipEmptyParts);
            if (lines.isEmpty())
                emit backend.openRequested(backend.homeUrl());
            for (const auto &line : lines)
                emit backend.openRequested(line);
        });
    });
    backend.setInitialUrls(urls);

    // The default QtWebEngine profile is off-the-record, so logins would vanish on exit.
    // A named profile with an explicit storage path keeps cookies, cache and site data on disk.
    const QString dataPath = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    auto *profile = new QQuickWebEngineProfile(&app);
    profile->setStorageName("nodaysidle-browser");
    profile->setOffTheRecord(false);
    profile->setPersistentStoragePath(dataPath + "/webengine");
    profile->setCachePath(dataPath + "/cache");
    profile->setPersistentCookiesPolicy(QQuickWebEngineProfile::ForcePersistentCookies);
    // Plain Chrome UA without the "QtWebEngine" token, which Google's sign-in rejects.
    // Chromium reduces the UA to major.0.0.0, so the version must match the engine.
    const QString chromeMajor = QString::fromLatin1(qWebEngineChromiumVersion()).section('.', 0, 0);
    profile->setHttpUserAgent(QString("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
                                      "Chrome/%1.0.0.0 Safari/537.36").arg(chromeMajor));
    profile->setHttpAcceptLanguage("en-US,en;q=0.9");
    profile->setPersistentPermissionsPolicy(QQuickWebEngineProfile::PersistentPermissionsPolicy::StoreInMemory);
    profile->setUrlRequestInterceptor(new AdBlocker(profile));

    QObject::connect(profile, &QQuickWebEngineProfile::downloadRequested, &backend,
                     [&backend](QQuickWebEngineDownloadRequest *download) {
        download->setDownloadDirectory(QStandardPaths::writableLocation(QStandardPaths::DownloadLocation));
        download->accept();
        QObject::connect(download, &QQuickWebEngineDownloadRequest::stateChanged, &backend,
                         [&backend, download](QQuickWebEngineDownloadRequest::DownloadState state) {
            if (state == QQuickWebEngineDownloadRequest::DownloadCompleted)
                emit backend.notice("Saved " + download->downloadFileName() + " to Downloads");
            else if (state == QQuickWebEngineDownloadRequest::DownloadInterrupted)
                emit backend.notice("Download failed: " + download->downloadFileName());
        });
    });

    QObject::connect(&app, &QCoreApplication::aboutToQuit, &backend, &Backend::flushHistory);

    Vault vault(dataPath + "/passwords.vault");
    Importer importer(&backend, &vault);

    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty("theme", &theme);
    engine.rootContext()->setContextProperty("backend", &backend);
    engine.rootContext()->setContextProperty("appProfile", profile);
    engine.rootContext()->setContextProperty("vault", &vault);
    engine.rootContext()->setContextProperty("importer", &importer);

    const QUrl url(QStringLiteral("qrc:/Main.qml"));
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreated,
                     &app, [url](QObject *obj, const QUrl &objUrl) {
        if (!obj && url == objUrl)
            QCoreApplication::exit(-1);
    }, Qt::QueuedConnection);

    engine.load(url);

    return app.exec();
}
