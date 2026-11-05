#pragma once

#include <QSet>
#include <QString>
#include <QUrl>
#include <QWebEngineUrlRequestInfo>
#include <QWebEngineUrlRequestInterceptor>
#include <atomic>

// Blocks sub-resource requests to well-known ad and tracking hosts.
// Page navigations are never blocked, and a site's own requests to itself pass.
class AdBlocker : public QWebEngineUrlRequestInterceptor {
    Q_OBJECT
public:
    explicit AdBlocker(QObject *parent = nullptr) : QWebEngineUrlRequestInterceptor(parent) {}

    int blockedCount() const { return m_blocked.load(); }

    static bool isBlockedHost(const QString &host) {
        static const QSet<QString> domains = {
            "doubleclick.net", "googlesyndication.com", "googleadservices.com", "googletagservices.com",
            "googletagmanager.com", "google-analytics.com", "adservice.google.com", "2mdn.net",
            "adnxs.com", "adsrvr.org", "advertising.com", "criteo.com", "criteo.net", "taboola.com",
            "outbrain.com", "scorecardresearch.com", "quantserve.com", "hotjar.com", "pubmatic.com",
            "rubiconproject.com", "openx.net", "casalemedia.com", "amazon-adsystem.com", "moatads.com",
            "adsafeprotected.com", "serving-sys.com", "smartadserver.com", "yieldmo.com", "teads.tv",
            "media.net", "zedo.com", "bidswitch.net", "contextweb.com", "lijit.com", "sharethrough.com",
            "tapad.com", "bluekai.com", "demdex.net", "krxd.net", "mathtag.com", "rlcdn.com",
            "everesttech.net", "mixpanel.com", "fullstory.com", "mouseflow.com", "crazyegg.com",
            "ads-twitter.com", "ads.linkedin.com", "analytics.tiktok.com", "ads.yahoo.com",
            "adsymptotic.com", "bat.bing.com", "popads.net", "propellerads.com", "exoclick.com",
        };
        for (QString candidate = host; candidate.contains('.'); candidate = candidate.mid(candidate.indexOf('.') + 1))
            if (domains.contains(candidate))
                return true;
        return false;
    }

    void interceptRequest(QWebEngineUrlRequestInfo &info) override {
        if (info.resourceType() == QWebEngineUrlRequestInfo::ResourceTypeMainFrame)
            return;
        const QString host = info.requestUrl().host().toLower();
        if (!isBlockedHost(host))
            return;
        const QString pageHost = info.firstPartyUrl().host().toLower();
        if (pageHost == host || pageHost.endsWith("." + host))
            return;
        info.block(true);
        ++m_blocked;
    }

private:
    std::atomic<int> m_blocked{0};
};
