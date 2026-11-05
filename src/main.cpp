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

#include "backend.h"
#include "theme.h"

int main(int argc, char *argv[]) {
    // 1. Force native Wayland platform (Zero XWayland / X11 overhead)
    qputenv("QT_QPA_PLATFORM", "wayland");
    qputenv("QT_WAYLAND_DISABLE_WINDOWDECORATION", "1");
    qputenv("NO_AT_BRIDGE", "1");

    // 2. Hardware acceleration flags for Intel Arc (Mesa iris driver + VA-API)
    qputenv("QTWEBENGINE_CHROMIUM_FLAGS",
            "--enable-features=VaapiVideoDecoder,VaapiVideoEncoder,CanvasOopRasterization "
            "--enable-gpu-rasterization "
            "--enable-zero-copy "
            "--use-gl=egl "
            "--ignore-gpu-blocklist "
            "--disable-smooth-scrolling");

    QtWebEngineQuick::initialize();

    QGuiApplication app(argc, argv);
    app.setApplicationName("nodaysidle-browser");
    app.setApplicationDisplayName("nodaysidle");
    app.setApplicationVersion("1.0.0");
    app.setDesktopFileName("com.nodaysidle.Browser");

    QQuickStyle::setStyle("Basic");

    Theme theme;
    Backend backend;

    // Configure default persistent web profile for smooth auth, cookies and cache
    auto *defaultProfile = QQuickWebEngineProfile::defaultProfile();
    const QString dataPath = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation) + "/nodaysidle-browser";
    defaultProfile->setPersistentStoragePath(dataPath);
    defaultProfile->setPersistentCookiesPolicy(QQuickWebEngineProfile::ForcePersistentCookies);
    defaultProfile->setHttpUserAgent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36");

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
