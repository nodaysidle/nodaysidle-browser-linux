#include <QGuiApplication>
#include <QLocalServer>
#include <QLocalSocket>
#include <memory>
#include <unistd.h>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QStandardPaths>
#include <QStyleHints>
#include <QtWebEngineQuick/qtwebenginequickglobal.h>
#include <QtWebEngineCore/qtwebenginecoreglobal.h>
#include <QQuickWebEngineProfile>

#include "adblock.h"
#include "backend.h"
#include "importer.h"
#include "theme.h"
#include "vault.h"

int main(int argc, char *argv[]) {
    // 1. Prefer native Wayland (zero XWayland overhead), but never override a platform the
    //    caller chose: offscreen and xcb are how this is driven in tests.
    if (qEnvironmentVariableIsEmpty("QT_QPA_PLATFORM"))
        qputenv("QT_QPA_PLATFORM", "wayland");
    qputenv("QT_WAYLAND_DISABLE_WINDOWDECORATION", "1");

    // Let QtWebEngine use native Wayland hardware acceleration; the renderer sandbox stays on.
    qputenv("QTWEBENGINE_CHROMIUM_FLAGS",
            "--ignore-gpu-blocklist "
            "--enable-gpu-rasterization "
            "--enable-zero-copy "
            + qgetenv("QTWEBENGINE_CHROMIUM_FLAGS"));

    QtWebEngineQuick::initialize();

    QGuiApplication app(argc, argv);
    app.setApplicationName("nodaysidle-browser");
    app.setApplicationDisplayName("nodaysidle");
    app.setApplicationVersion("0.4.0");
    app.setDesktopFileName("com.nodaysidle.Browser");

    QQuickStyle::setStyle("Basic");

    QStringList args;
    for (int i = 1; i < argc; ++i)
        if (argv[i][0] != '-')
            args << QString::fromUtf8(argv[i]);

    Theme theme;
    Backend backend;

    // Follow the theme's mode so pages' prefers-color-scheme matches the desktop.
    const auto applyColorScheme = [&app, &theme] {
        app.styleHints()->setColorScheme(theme.dark() ? Qt::ColorScheme::Dark : Qt::ColorScheme::Light);
    };
    QObject::connect(&theme, &Theme::changed, &app, applyColorScheme);
    applyColorScheme();
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
    if (!server.listen(serverName))
        qWarning("nodaysidle: could not listen on %s (%s); links from other apps will start a new window",
                 qPrintable(serverName), qPrintable(server.errorString()));
    QObject::connect(&server, &QLocalServer::newConnection, &backend, [&server, &backend] {
        auto *socket = server.nextPendingConnection();
        // A URL can arrive split across reads, so only complete lines are handed over.
        auto buffer = std::make_shared<QByteArray>();
        auto openedSomething = std::make_shared<bool>(false);
        QObject::connect(socket, &QLocalSocket::disconnected, &backend,
                         [socket, buffer, openedSomething, &backend] {
            const QByteArray rest = buffer->trimmed();
            if (!rest.isEmpty()) {
                emit backend.openRequested(QString::fromUtf8(rest));
                *openedSomething = true;
            } else if (!*openedSomething) {
                emit backend.openRequested(backend.homeUrl());
            }
            socket->deleteLater();
        });
        QObject::connect(socket, &QLocalSocket::readyRead, &backend, [socket, buffer, openedSomething, &backend] {
            buffer->append(socket->readAll());
            qsizetype newline;
            while ((newline = buffer->indexOf('\n')) >= 0) {
                const QByteArray line = buffer->left(newline).trimmed();
                buffer->remove(0, newline + 1);
                if (line.isEmpty())
                    continue;
                emit backend.openRequested(QString::fromUtf8(line));
                *openedSomething = true;
            }
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

    // Downloads are handled in QML (WebEngineProfile.downloadRequested): nothing is written
    // to disk until the person presses Save in the shelf.

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
