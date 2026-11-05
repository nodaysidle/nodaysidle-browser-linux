#include <QGuiApplication>
#include <QIcon>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QStandardPaths>
#include <QtWebEngineQuick/qtwebenginequickglobal.h>
#include <QWebEngineProfile>
#include <QWebEngineSettings>
#include <QQuickWebEngineProfile>

#include "auth_interceptor.h"
#include "backend.h"
#include "theme.h"

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
            "--no-sandbox");

    QtWebEngineQuick::initialize();

    QGuiApplication app(argc, argv);
    app.setApplicationName("nodaysidle-browser");
    app.setApplicationDisplayName("nodaysidle");
    app.setApplicationVersion("1.0.0");
    app.setDesktopFileName("com.nodaysidle.Browser");

    QQuickStyle::setStyle("Basic");

    Theme theme;
    Backend backend;
    if (argc > 1 && argv[1][0] != '-') {
        backend.setInitialUrl(backend.normalizeUrl(QString::fromUtf8(argv[1])));
    }

    // Configure default persistent web profile for smooth auth, cookies and cache
    auto *defaultProfile = QQuickWebEngineProfile::defaultProfile();
    const QString dataPath = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    defaultProfile->setPersistentStoragePath(dataPath);
    defaultProfile->setCachePath(dataPath + "/cache");
    defaultProfile->setPersistentCookiesPolicy(QQuickWebEngineProfile::ForcePersistentCookies);
    defaultProfile->setHttpUserAgent("Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0");
    defaultProfile->setUrlRequestInterceptor(new GoogleAuthInterceptor(defaultProfile));

    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty("theme", &theme);
    engine.rootContext()->setContextProperty("backend", &backend);

    const QUrl url(QStringLiteral("qrc:/Main.qml"));
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreated,
                     &app, [url](QObject *obj, const QUrl &objUrl) {
        if (!obj && url == objUrl)
            QCoreApplication::exit(-1);
    }, Qt::QueuedConnection);

    engine.load(url);

    return app.exec();
}
