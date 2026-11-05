#include "backend.h"

#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QClipboard>
#include <QFileInfo>
#include <QGuiApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkRequest>
#include <QRegularExpression>
#include <QSet>
#include <QStandardPaths>
#include <QUrl>
#include <QUrlQuery>
#include <memory>

Backend::Backend(QObject *parent) : QObject(parent) {
    loadKeys();
    loadBookmarks();
    loadHistory();
    m_historySave.setSingleShot(true);
    m_historySave.setInterval(2000);
    connect(&m_historySave, &QTimer::timeout, this, &Backend::flushHistory);
}

static QString historyPath() {
    return QStandardPaths::writableLocation(QStandardPaths::AppDataLocation) + "/history.json";
}

void Backend::loadHistory() {
    QFile file(historyPath());
    if (!file.open(QIODevice::ReadOnly))
        return;
    const auto doc = QJsonDocument::fromJson(file.readAll());
    for (const auto &entry : doc.array())
        m_history.append(entry.toObject().toVariantMap());
}

void Backend::flushHistory() {
    m_historySave.stop();
    QDir().mkpath(QFileInfo(historyPath()).absolutePath());
    QFile file(historyPath());
    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate))
        return;
    file.setPermissions(QFile::ReadOwner | QFile::WriteOwner);
    QJsonArray array;
    for (const auto &entry : m_history)
        array.append(QJsonObject::fromVariantMap(entry.toMap()));
    file.write(QJsonDocument(array).toJson(QJsonDocument::Compact));
}

void Backend::recordVisit(const QString &url, const QString &title) {
    if (!url.startsWith("http://") && !url.startsWith("https://"))
        return;
    const qint64 now = QDateTime::currentSecsSinceEpoch();
    // Same page again (reload, or the title arriving late): refresh the newest entry.
    if (!m_history.isEmpty()) {
        QVariantMap last = m_history.last().toMap();
        if (last["url"].toString() == url) {
            if (!title.isEmpty())
                last["title"] = title;
            m_history.last() = last;
            m_historySave.start();
            return;
        }
    }
    QVariantMap entry;
    entry["url"] = url;
    entry["title"] = title.isEmpty() ? url : title;
    entry["time"] = now;
    m_history.append(entry);
    constexpr int maxEntries = 5000;
    if (m_history.size() > maxEntries)
        m_history = m_history.mid(m_history.size() - maxEntries);
    m_historySave.start();
    emit historyChanged();
}

QVariantList Backend::searchHistory(const QString &query, int limit) {
    QVariantList result;
    QSet<QString> seen;
    const QString needle = query.trimmed().toLower();
    for (int i = m_history.size() - 1; i >= 0 && result.size() < limit; --i) {
        const QVariantMap entry = m_history[i].toMap();
        const QString url = entry["url"].toString();
        if (!needle.isEmpty() && !url.toLower().contains(needle) && !entry["title"].toString().toLower().contains(needle))
            continue;
        if (needle.isEmpty() || !seen.contains(url)) {
            seen.insert(url);
            result.append(entry);
        }
    }
    return result;
}

QVariantList Backend::suggest(const QString &query, int limit) {
    QVariantList result;
    const QString needle = query.trimmed().toLower();
    if (needle.isEmpty())
        return result;
    QSet<QString> seen;
    auto consider = [&](const QVariantMap &entry) {
        const QString url = entry["url"].toString();
        if (seen.contains(url) || result.size() >= limit)
            return;
        if (url.toLower().contains(needle) || entry["title"].toString().toLower().contains(needle)) {
            seen.insert(url);
            result.append(entry);
        }
    };
    for (const auto &bookmark : m_bookmarks)
        consider(bookmark.toMap());
    for (int i = m_history.size() - 1; i >= 0; --i)
        consider(m_history[i].toMap());
    return result;
}

void Backend::clearHistory() {
    m_history.clear();
    flushHistory();
    emit historyChanged();
}

QString Backend::homeUrl() const {
    return "qrc:/home.html";
}

void Backend::loadKeys() {
    m_anthropicKey = qEnvironmentVariable("ANTHROPIC_API_KEY");
    if (m_anthropicKey.isEmpty())
        m_anthropicKey = qEnvironmentVariable("HAIKU_API_KEY");
    if (m_anthropicKey.isEmpty())
        m_anthropicKey = findKeyInFish("HAIKU_API_KEY");
    if (m_anthropicKey.isEmpty())
        m_anthropicKey = findKeyInFish("ANTHROPIC_API_KEY");

    m_jevKey = qEnvironmentVariable("TYPESAFE_API_KEY");
    if (m_jevKey.isEmpty())
        m_jevKey = findKeyInFish("TYPESAFE_API_KEY");
}

QString Backend::findKeyInFish(const QString &target) {
    const QString home = QDir::homePath();
    // 1. secrets.fish
    QFile secrets(home + "/.config/fish/secrets.fish");
    if (secrets.open(QIODevice::ReadOnly | QIODevice::Text)) {
        while (!secrets.atEnd()) {
            QString line = QString::fromUtf8(secrets.readLine()).trimmed();
            QString prefix = QString("set -gx %1 ").arg(target);
            if (line.startsWith(prefix)) {
                QString val = line.mid(prefix.length()).trimmed();
                if ((val.startsWith('"') && val.endsWith('"')) || (val.startsWith('\'') && val.endsWith('\'')))
                    val = val.mid(1, val.length() - 2);
                if (!val.isEmpty())
                    return val;
            }
        }
    }

    // 2. fish_variables
    QFile vars(home + "/.config/fish/fish_variables");
    if (vars.open(QIODevice::ReadOnly | QIODevice::Text)) {
        while (!vars.atEnd()) {
            QString line = QString::fromUtf8(vars.readLine());
            if (line.contains(" " + target + ":")) {
                int idx = line.indexOf(" " + target + ":") + 1;
                QString val = line.mid(idx + target.length() + 1).trimmed();
                if (!val.isEmpty())
                    return val;
            }
        }
    }

    return QString();
}

void Backend::loadBookmarks() {
    m_bookmarks.clear();
    QFile file(QStandardPaths::writableLocation(QStandardPaths::AppDataLocation) + "/bookmarks.json");
    if (file.open(QIODevice::ReadOnly)) {
        const auto doc = QJsonDocument::fromJson(file.readAll());
        if (doc.isArray()) {
            for (const auto &val : doc.array()) {
                m_bookmarks.append(val.toObject().toVariantMap());
            }
            emit bookmarksChanged();
            return;
        }
    }

    // Default bookmarks
    QVariantMap gemini, google, github, youtube;
    gemini["title"] = "Gemini";
    gemini["url"] = "https://gemini.google.com";
    google["title"] = "Google";
    google["url"] = "https://google.com";
    github["title"] = "GitHub";
    github["url"] = "https://github.com";
    youtube["title"] = "YouTube";
    youtube["url"] = "https://youtube.com";

    m_bookmarks = {gemini, google, github, youtube};
    saveBookmarks();
    emit bookmarksChanged();
}

void Backend::saveBookmarks() {
    const QString dataPath = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    QDir().mkpath(dataPath);
    QFile file(dataPath + "/bookmarks.json");
    if (file.open(QIODevice::WriteOnly)) {
        QJsonArray arr;
        for (const auto &v : m_bookmarks) {
            arr.append(QJsonObject::fromVariantMap(v.toMap()));
        }
        file.write(QJsonDocument(arr).toJson(QJsonDocument::Indented));
    }
}

bool Backend::isBookmarked(const QString &url) {
    const QString norm = url.trimmed().toLower();
    for (const auto &b : m_bookmarks) {
        if (b.toMap()["url"].toString().trimmed().toLower() == norm)
            return true;
    }
    return false;
}

void Backend::toggleBookmark(const QString &title, const QString &url) {
    const QString norm = url.trimmed().toLower();
    if (norm.isEmpty() || norm == "about:blank" || norm.startsWith("qrc:"))
        return;

    for (int i = 0; i < m_bookmarks.size(); ++i) {
        if (m_bookmarks[i].toMap()["url"].toString().trimmed().toLower() == norm) {
            m_bookmarks.removeAt(i);
            saveBookmarks();
            emit bookmarksChanged();
            return;
        }
    }

    QVariantMap bm;
    bm["title"] = title.trimmed().isEmpty() ? url : title.trimmed();
    bm["url"] = url.trimmed();
    m_bookmarks.append(bm);
    saveBookmarks();
    emit bookmarksChanged();
}

void Backend::removeBookmark(const QString &url) {
    const QString norm = url.trimmed().toLower();
    for (int i = 0; i < m_bookmarks.size(); ++i) {
        if (m_bookmarks[i].toMap()["url"].toString().trimmed().toLower() == norm) {
            m_bookmarks.removeAt(i);
            saveBookmarks();
            emit bookmarksChanged();
            return;
        }
    }
}

QString Backend::readResource(const QString &path) const {
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? QString::fromUtf8(file.readAll()) : QString();
}

QString Backend::pdfPath(const QString &title) const {
    static const QRegularExpression unsafe("[^A-Za-z0-9._ -]");
    QString name = title.trimmed();
    name.replace(unsafe, "_");
    if (name.isEmpty())
        name = "page";
    const QDir dir(QStandardPaths::writableLocation(QStandardPaths::DownloadLocation));
    QString path = dir.filePath(name + ".pdf");
    for (int n = 1; QFileInfo::exists(path); ++n)
        path = dir.filePath(QString("%1 (%2).pdf").arg(name).arg(n));
    return path;
}

void Backend::copyText(const QString &text) {
    QGuiApplication::clipboard()->setText(text);
}

QString Backend::normalizeUrl(const QString &input) {
    const QString trimmed = input.trimmed();
    if (trimmed.isEmpty())
        return "about:blank";

    if (trimmed.startsWith("http://") || trimmed.startsWith("https://") || trimmed.startsWith("about:") ||
        trimmed.startsWith("qrc:") || trimmed.startsWith("file:"))
        return trimmed;

    const bool pathLike = trimmed.startsWith('/') || trimmed.startsWith("./") || trimmed.startsWith("../") || trimmed.startsWith("~/");
    if (pathLike) {
        const QString path = trimmed.startsWith("~/") ? QDir::homePath() + trimmed.mid(1) : trimmed;
        if (QFileInfo::exists(path))
            return QUrl::fromLocalFile(QFileInfo(path).absoluteFilePath()).toString();
    }

    // Local servers speak plain http: localhost, IP literals and host:port.
    static const QRegularExpression local(
        R"(^(localhost|\d{1,3}(\.\d{1,3}){3}|\[[0-9a-fA-F:]+\]|[^\s/:]+:\d+)(:\d+)?([/?#]\S*)?$)");
    if (local.match(trimmed).hasMatch())
        return "http://" + trimmed;

    // Check if it looks like a domain (has dot and no whitespace)
    if (!trimmed.contains(' ') && trimmed.contains('.'))
        return "https://" + trimmed;

    // Otherwise, treat as search query (DuckDuckGo / Google)
    QUrl searchUrl("https://duckduckgo.com/");
    QUrlQuery query;
    query.addQueryItem("q", trimmed);
    searchUrl.setQuery(query);
    return searchUrl.toString();
}

void Backend::cancelAssistant() {
    if (m_activeReply) {
        auto *reply = m_activeReply;
        m_activeReply = nullptr;
        reply->abort();
    }
}

void Backend::askAssistant(const QString &title, const QString &url, const QString &text, const QString &prompt) {
    cancelAssistant();

    if (m_anthropicKey.isEmpty()) {
        emit assistantError("Set ANTHROPIC_API_KEY or HAIKU_API_KEY in your shell.");
        return;
    }

    QNetworkRequest request(QUrl("https://api.anthropic.com/v1/messages"));
    request.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");
    request.setRawHeader("x-api-key", m_anthropicKey.toUtf8());
    request.setRawHeader("anthropic-version", "2023-06-01");

    QJsonObject body;
    body["model"] = "claude-haiku-5-5";
    body["max_tokens"] = 4000;
    body["stream"] = true;

    QString truncatedText = text.left(12000);
    QString system = QString("You answer questions about the web page below for the person reading it. Be concise, direct and helpful.\n\nTitle: %1\nURL: %2\n<page>\n%3\n</page>")
                         .arg(title, url, truncatedText);
    body["system"] = system;

    QJsonArray messages;
    QJsonObject msg;
    msg["role"] = "user";
    msg["content"] = prompt;
    messages.append(msg);
    body["messages"] = messages;

    QNetworkReply *reply = m_nam.post(request, QJsonDocument(body).toJson());
    m_activeReply = reply;

    auto pending = std::make_shared<QByteArray>();
    connect(reply, &QNetworkReply::readyRead, this, [this, reply, pending]() {
        pending->append(reply->readAll());
        qsizetype newline;
        while ((newline = pending->indexOf('\n')) >= 0) {
            const QByteArray line = pending->left(newline).trimmed();
            pending->remove(0, newline + 1);
            if (!line.startsWith("data: "))
                continue;
            const auto obj = QJsonDocument::fromJson(line.mid(6)).object();
            if (obj["type"].toString() == "content_block_delta") {
                const QString chunk = obj["delta"].toObject()["text"].toString();
                if (!chunk.isEmpty())
                    emit assistantChunk(chunk);
            }
        }
    });

    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        if (reply == m_activeReply)
            m_activeReply = nullptr;
        if (reply->error() == QNetworkReply::OperationCanceledError) {
            emit assistantDone();
        } else if (reply->error() != QNetworkReply::NoError) {
            const QString detail = QJsonDocument::fromJson(reply->readAll()).object()["error"].toObject()["message"].toString();
            emit assistantError(detail.isEmpty() ? reply->errorString() : detail);
        } else {
            emit assistantDone();
        }
        reply->deleteLater();
    });
}

void Backend::checkScam(const QString &title, const QString &url, const QString &text) {
    if (m_jevKey.isEmpty()) {
        emit assistantError("Set TYPESAFE_API_KEY in your shell.");
        return;
    }

    QUrl qurl(url);
    QString host = qurl.host();

    QNetworkRequest request(QUrl("https://api.typesafe.ai/v1/systemone"));
    request.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");
    request.setRawHeader("Authorization", ("Bearer " + m_jevKey).toUtf8());

    QJsonObject state;
    state["host"] = host;
    state["title"] = title;
    state["text"] = text.left(12000);

    QJsonObject questions;
    QJsonObject q1, q2, q3;
    q1["type"] = "noul";
    q1["instructions"] = "Does the page in `state` present itself as a well-known brand, bank or service that its `host` does not belong to?";
    q2["type"] = "noul";
    q2["instructions"] = "Does the page in `state` ask the visitor for a password, one-time code, payment card or account recovery details?";
    q3["type"] = "noul";
    q3["instructions"] = "Does the page in `state` use threats, deadlines or alarming warnings to rush the visitor into acting?";
    questions["impersonation"] = q1;
    questions["credentials"] = q2;
    questions["pressure"] = q3;

    QJsonObject body;
    body["model"] = "jev-latest";
    body["state"] = state;
    body["questions"] = questions;

    auto *reply = m_nam.post(request, QJsonDocument(body).toJson());
    connect(reply, &QNetworkReply::finished, this, [this, reply, host]() {
        if (reply->error() == QNetworkReply::NoError) {
            auto doc = QJsonDocument::fromJson(reply->readAll());
            auto answers = doc.object()["answers"].toObject();
            double imp = answers["impersonation"].toObject()["noul"].toDouble(0.0);
            double cred = answers["credentials"].toObject()["noul"].toDouble(0.0);
            double press = answers["pressure"].toObject()["noul"].toDouble(0.0);

            QString verdict = "No strong scam signals.";
            if (imp >= 0.7 && (cred >= 0.5 || press >= 0.7)) {
                verdict = "High risk: looks like a scam!";
            } else if (imp >= 0.7 || press >= 0.7) {
                verdict = "Be careful: some scam signals detected.";
            }

            emit scamResult(verdict, qRound(imp * 100), qRound(cred * 100), qRound(press * 100), host);
        } else {
            emit assistantError("TypeSafe Jev request failed: " + reply->errorString());
        }
        reply->deleteLater();
    });
}
