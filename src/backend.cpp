#include "backend.h"

#include <QDateTime>
#include <QDesktopServices>
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
#include <QSaveFile>
#include <QSet>
#include <QStandardPaths>
#include <QUrl>
#include <QUrlQuery>
#include <algorithm>

Backend::Backend(QObject *parent) : QObject(parent) {
    loadKeys();
    loadBookmarks();
    loadHistory();
    loadSession();
    m_historySave.setSingleShot(true);
    m_historySave.setInterval(2000);
    connect(&m_historySave, &QTimer::timeout, this, &Backend::flushHistory);
}

static QString historyPath() {
    return QStandardPaths::writableLocation(QStandardPaths::AppDataLocation) + "/history.json";
}

static QString sessionPath() {
    return QStandardPaths::writableLocation(QStandardPaths::AppDataLocation) + "/session.json";
}

void Backend::loadSession() {
    QFile file(sessionPath());
    if (!file.open(QIODevice::ReadOnly))
        return;
    const QJsonObject root = QJsonDocument::fromJson(file.readAll()).object();
    for (const auto &value : root.value("urls").toArray()) {
        const QString url = value.toString();
        if (!url.isEmpty())
            m_sessionTabs.append(url);
    }
    m_sessionIndex = root.value("index").toInt(0);
}

void Backend::saveSession(const QVariantList &urls, int current) {
    QDir().mkpath(QFileInfo(sessionPath()).absolutePath());
    QSaveFile file(sessionPath());
    if (!file.open(QIODevice::WriteOnly))
        return;
    QJsonObject root;
    root["urls"] = QJsonArray::fromVariantList(urls);
    root["index"] = current;
    file.write(QJsonDocument(root).toJson(QJsonDocument::Compact));
    if (file.commit())
        QFile::setPermissions(sessionPath(), QFile::ReadOwner | QFile::WriteOwner);
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
    QSaveFile file(historyPath());
    if (!file.open(QIODevice::WriteOnly))
        return;
    QJsonArray array;
    for (const auto &entry : m_history)
        array.append(QJsonObject::fromVariantMap(entry.toMap()));
    file.write(QJsonDocument(array).toJson(QJsonDocument::Compact));
    if (file.commit())
        QFile::setPermissions(historyPath(), QFile::ReadOwner | QFile::WriteOwner);
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

int Backend::addImportedHistory(const QVariantList &items) {
    QSet<QString> known;
    for (const auto &entry : m_history) {
        const QVariantMap e = entry.toMap();
        known.insert(e["url"].toString() + '\n' + e["time"].toString());
    }
    int added = 0;
    for (const auto &item : items) {
        QVariantMap entry = item.toMap();
        const QString key = entry["url"].toString() + '\n' + entry["time"].toString();
        if (entry["url"].toString().isEmpty() || entry["time"].toLongLong() <= 0 || known.contains(key))
            continue;
        if (entry["title"].toString().isEmpty())
            entry["title"] = entry["url"];
        known.insert(key);
        m_history.append(entry);
        ++added;
    }
    if (added == 0)
        return 0;
    std::stable_sort(m_history.begin(), m_history.end(), [](const QVariant &a, const QVariant &b) {
        return a.toMap()["time"].toLongLong() < b.toMap()["time"].toLongLong();
    });
    constexpr int maxEntries = 5000;
    if (m_history.size() > maxEntries)
        m_history = m_history.mid(m_history.size() - maxEntries);
    flushHistory();
    emit historyChanged();
    return added;
}

int Backend::addImportedBookmarks(const QString &folder, const QVariantList &items) {
    QSet<QString> known;
    for (const auto &b : m_bookmarks)
        known.insert(b.toMap()["url"].toString().trimmed().toLower());
    int added = 0;
    for (const auto &item : items) {
        const QVariantMap source = item.toMap();
        const QString url = source["url"].toString().trimmed();
        if ((!url.startsWith("http://") && !url.startsWith("https://")) || known.contains(url.toLower()))
            continue;
        known.insert(url.toLower());
        const QString title = source["title"].toString().trimmed();
        m_bookmarks.append(QVariantMap{{"title", title.isEmpty() ? url : title}, {"url", url}, {"folder", folder}});
        ++added;
    }
    if (added > 0) {
        saveBookmarks();
        emit bookmarksChanged();
    }
    return added;
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
    // Keep the list in memory so the toast's Undo can put it back.
    m_historyBackup = m_history;
    m_history.clear();
    flushHistory();
    emit historyChanged();
}

void Backend::undoClearHistory() {
    if (m_historyBackup.isEmpty())
        return;
    m_history = m_historyBackup;
    m_historyBackup.clear();
    flushHistory();
    emit historyChanged();
}

QString Backend::homeUrl() const {
    return "qrc:/home.html";
}

void Backend::loadKeys() {
    // TypeSafe is the only AI integration left, and its key comes from the environment.
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
    if (!file.open(QIODevice::ReadOnly))
        return;
    for (const auto &val : QJsonDocument::fromJson(file.readAll()).array())
        m_bookmarks.append(val.toObject().toVariantMap());

    // Earlier versions seeded these four; the bar now holds only what the user adds or imports.
    static const QSet<QString> seeded{"Gemini|https://gemini.google.com", "Google|https://google.com",
                                      "GitHub|https://github.com", "YouTube|https://youtube.com"};
    const int before = m_bookmarks.size();
    m_bookmarks.erase(std::remove_if(m_bookmarks.begin(), m_bookmarks.end(), [](const QVariant &b) {
        const QVariantMap m = b.toMap();
        return !m.contains("folder") && seeded.contains(m["title"].toString() + "|" + m["url"].toString());
    }), m_bookmarks.end());
    if (m_bookmarks.size() != before)
        saveBookmarks();
}

void Backend::saveBookmarks() {
    const QString dataPath = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    QDir().mkpath(dataPath);
    QSaveFile file(dataPath + "/bookmarks.json");
    if (file.open(QIODevice::WriteOnly)) {
        QJsonArray arr;
        for (const auto &v : m_bookmarks) {
            arr.append(QJsonObject::fromVariantMap(v.toMap()));
        }
        file.write(QJsonDocument(arr).toJson(QJsonDocument::Indented));
        if (file.commit())
            QFile::setPermissions(dataPath + "/bookmarks.json", QFile::ReadOwner | QFile::WriteOwner);
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
            m_bookmarkBackup = m_bookmarks[i].toMap();
            m_bookmarkBackupValid = true;
            m_bookmarks.removeAt(i);
            saveBookmarks();
            emit bookmarksChanged();
            return;
        }
    }
}

void Backend::undoRemoveBookmark() {
    if (!m_bookmarkBackupValid)
        return;
    m_bookmarks.append(m_bookmarkBackup);
    m_bookmarkBackupValid = false;
    m_bookmarkBackup.clear();
    saveBookmarks();
    emit bookmarksChanged();
}

QString Backend::readResource(const QString &path) const {
    // Only the app's own resources: nothing may turn this into a file reader.
    if (!path.startsWith(":/"))
        return QString();
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

QString Backend::downloadDir() const {
    return QStandardPaths::writableLocation(QStandardPaths::DownloadLocation);
}

void Backend::openPath(const QString &path) const {
    QDesktopServices::openUrl(QUrl::fromLocalFile(path));
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

namespace {

// TypeSafe reports failures in the response body, so Qt's errorString() alone hides the reason.
QString typesafeError(int status, const QByteArray &body, const QString &networkError) {
    QString detail;
    const QJsonObject root = QJsonDocument::fromJson(body).object();
    const QJsonValue error = root.value("error");
    if (error.isObject()) {
        detail = error.toObject().value("message").toString();
        if (detail.isEmpty())
            detail = error.toObject().value("type").toString();
    } else if (error.isString()) {
        detail = error.toString();
    }
    if (detail.isEmpty())
        detail = root.value("message").toString();
    if (detail.isEmpty())
        detail = QString::fromUtf8(body).trimmed().left(300);

    QString summary;
    if (status == 401 || status == 403)
        summary = QString("TypeSafe API key rejected (%1): check TYPESAFE_API_KEY").arg(status);
    else if (status == 402)
        summary = "TypeSafe is out of credits or has a billing problem (402): top up or unset TYPESAFE_API_KEY";
    else if (status == 429)
        summary = "TypeSafe rate or spend limit reached (429): wait and retry";
    else if (status >= 500)
        summary = QString("TypeSafe server error (%1): retry in a moment").arg(status);
    else if (status >= 400)
        summary = QString("TypeSafe rejected the request (%1)").arg(status);
    else
        summary = "TypeSafe request failed: " + (networkError.isEmpty() ? QStringLiteral("no response") : networkError);

    return detail.isEmpty() ? summary : summary + "\n" + detail.left(300);
}

}  // namespace

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
        const int status = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
        // Read the body once: the error payload is the only place TypeSafe names the cause.
        const QByteArray payload = reply->readAll();
        const QJsonObject root = QJsonDocument::fromJson(payload).object();

        // A 2xx carrying an error object, or a body without answers, is a failure as well.
        if (reply->error() != QNetworkReply::NoError || status >= 400 || root.contains("error")) {
            emit assistantError(typesafeError(status, payload, reply->errorString()));
            reply->deleteLater();
            return;
        }
        const QJsonObject answers = root.value("answers").toObject();
        if (!answers.contains("impersonation") || !answers.contains("credentials") || !answers.contains("pressure")) {
            emit assistantError(typesafeError(status, payload, "the response carried no answers"));
            reply->deleteLater();
            return;
        }

        const double imp = answers.value("impersonation").toObject().value("noul").toDouble(0.0);
        const double cred = answers.value("credentials").toObject().value("noul").toDouble(0.0);
        const double press = answers.value("pressure").toObject().value("noul").toDouble(0.0);

        QString verdict = "No strong scam signals.";
        if (imp >= 0.7 && (cred >= 0.5 || press >= 0.7)) {
            verdict = "High risk: looks like a scam!";
        } else if (imp >= 0.7 || press >= 0.7) {
            verdict = "Be careful: some scam signals detected.";
        }

        emit scamResult(verdict, qRound(imp * 100), qRound(cred * 100), qRound(press * 100), host);
        reply->deleteLater();
    });
}
