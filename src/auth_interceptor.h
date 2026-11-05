#pragma once

#include <QWebEngineUrlRequestInterceptor>
#include <QWebEngineUrlRequestInfo>
#include <QUrl>
#include <QString>

class GoogleAuthInterceptor : public QWebEngineUrlRequestInterceptor {
    Q_OBJECT
public:
    explicit GoogleAuthInterceptor(QObject *parent = nullptr)
        : QWebEngineUrlRequestInterceptor(parent) {}

    void interceptRequest(QWebEngineUrlRequestInfo &info) override {
        const QString host = info.requestUrl().host();
        // Google blocks Chromium-based embedded webviews (QtWebEngine/CEF/Electron)
        // that claim to be Google Chrome on Linux because they lack Google Chrome's
        // proprietary internal binary tokens.
        // Spoofing modern Firefox for Google accounts & auth endpoints bypasses the block.
        if (host == "accounts.google.com" ||
            host == "accounts.youtube.com" ||
            host == "myaccount.google.com" ||
            host.endsWith(".google.com")) {
            info.setHttpHeader("User-Agent", "Mozilla/5.0 (X11; Linux x86_64; rv:135.0) Gecko/20100101 Firefox/135.0");
        }
    }
};
