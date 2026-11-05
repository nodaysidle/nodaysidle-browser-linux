#include "importer.h"

#include "backend.h"
#include "vault.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLocale>
#include <QProcess>
#include <QRegularExpression>
#include <QSettings>
#include <QSqlDatabase>
#include <QSqlQuery>
#include <QTemporaryDir>
#include <QUrl>
#include <openssl/evp.h>

namespace {

struct BrowserKind {
    QString key;
    QString name;
    QString root;         // relative to the home folder
    bool firefox;
    QString keyringApp;   // the "application" attribute of the Chromium "Safe Storage" keyring item
};

const QList<BrowserKind> &kinds() {
    static const QList<BrowserKind> list{
        {"chrome", "Chrome", ".config/google-chrome", false, "chrome"},
        {"chromium", "Chromium", ".config/chromium", false, "chromium"},
        {"brave", "Brave", ".config/BraveSoftware/Brave-Browser", false, "brave"},
        {"edge", "Edge", ".config/microsoft-edge", false, "microsoft-edge"},
        {"vivaldi", "Vivaldi", ".config/vivaldi", false, "vivaldi"},
        {"opera", "Opera", ".config/opera", false, "opera"},
        {"firefox", "Firefox", ".mozilla/firefox", true, {}},
        {"zen", "Zen", ".zen", true, {}},
        {"zen2", "Zen", ".config/zen", true, {}},
        {"librewolf", "LibreWolf", ".librewolf", true, {}},
        {"floorp", "Floorp", ".floorp", true, {}},
        {"waterfox", "Waterfox", ".waterfox", true, {}},
    };
    return list;
}

constexpr int maxHistory = 5000;

struct Profile {
    QString dir;
    QString name;
};

QList<Profile> chromiumProfiles(const QString &root) {
    QJsonObject cache;
    QFile state(root + "/Local State");
    if (state.open(QIODevice::ReadOnly))
        cache = QJsonDocument::fromJson(state.readAll()).object().value("profile").toObject().value("info_cache").toObject();
    QList<Profile> profiles;
    static const QRegularExpression profileDir("^(Default|Profile \\d+)$");
    for (const QString &name : QDir(root).entryList(QDir::Dirs | QDir::NoDotAndDotDot, QDir::Name)) {
        if (!profileDir.match(name).hasMatch())
            continue;
        const QString dir = root + "/" + name;
        if (!QFileInfo::exists(dir + "/Bookmarks") && !QFileInfo::exists(dir + "/History") && !QFileInfo::exists(dir + "/Login Data"))
            continue;
        const QString shown = cache.value(name).toObject().value("name").toString();
        profiles.append({dir, shown.isEmpty() ? name : shown});
    }
    return profiles;
}

QList<Profile> firefoxProfiles(const QString &root) {
    QList<Profile> profiles;
    QSettings ini(root + "/profiles.ini", QSettings::IniFormat);
    QSet<QString> seen;
    for (const QString &group : ini.childGroups()) {
        if (!group.startsWith("Profile"))
            continue;
        ini.beginGroup(group);
        const QString path = ini.value("Path").toString();
        const bool relative = ini.value("IsRelative", 1).toInt() == 1;
        const QString name = ini.value("Name", path).toString();
        ini.endGroup();
        const QString dir = QDir::cleanPath(relative ? root + "/" + path : path);
        if (!path.isEmpty() && !seen.contains(dir) && QFileInfo::exists(dir + "/places.sqlite")) {
            seen.insert(dir);
            profiles.append({dir, name});
        }
    }
    return profiles;
}

// Browsers keep their databases locked while they run, so read a private copy (with its journal).
QList<QVariantList> readRows(const QString &dbFile, const QString &sql, int columns) {
    QList<QVariantList> rows;
    QTemporaryDir temp;
    if (!temp.isValid() || !QFileInfo::exists(dbFile))
        return rows;
    const QString copy = temp.filePath("db.sqlite");
    if (!QFile::copy(dbFile, copy))
        return rows;
    for (const QString &suffix : {"-wal", "-shm"})
        if (QFileInfo::exists(dbFile + suffix))
            QFile::copy(dbFile + suffix, copy + suffix);

    const QString connection = "nodaysidle-import-" + QString::number(reinterpret_cast<quintptr>(&rows));
    {
        QSqlDatabase db = QSqlDatabase::addDatabase("QSQLITE", connection);
        db.setDatabaseName(copy);
        db.setConnectOptions("QSQLITE_OPEN_READONLY");
        if (db.open()) {
            QSqlQuery query(db);
            if (query.exec(sql)) {
                while (query.next()) {
                    QVariantList row;
                    for (int i = 0; i < columns; ++i)
                        row.append(query.value(i));
                    rows.append(row);
                }
            }
            db.close();
        }
    }
    QSqlDatabase::removeDatabase(connection);
    return rows;
}

void collectChromiumBookmarks(const QJsonObject &node, QVariantList *out) {
    if (node.value("type").toString() == "url") {
        out->append(QVariantMap{{"title", node.value("name").toString()}, {"url", node.value("url").toString()}});
        return;
    }
    for (const auto &child : node.value("children").toArray())
        collectChromiumBookmarks(child.toObject(), out);
}

QByteArray pbkdf2(const QByteArray &password) {
    QByteArray key(16, 0);
    PKCS5_PBKDF2_HMAC(password.constData(), password.size(), reinterpret_cast<const unsigned char *>("saltysalt"), 9, 1,
                      EVP_sha1(), key.size(), reinterpret_cast<unsigned char *>(key.data()));
    return key;
}

bool decryptChromium(const QByteArray &key, const QByteArray &cipher, QString *plain) {
    const QByteArray iv(16, ' ');
    QByteArray out(cipher.size() + 16, 0);
    int len = 0;
    int total = 0;
    EVP_CIPHER_CTX *ctx = EVP_CIPHER_CTX_new();
    bool ok = EVP_DecryptInit_ex(ctx, EVP_aes_128_cbc(), nullptr, reinterpret_cast<const unsigned char *>(key.constData()),
                                 reinterpret_cast<const unsigned char *>(iv.constData())) == 1
        && EVP_DecryptUpdate(ctx, reinterpret_cast<unsigned char *>(out.data()), &len,
                             reinterpret_cast<const unsigned char *>(cipher.constData()), cipher.size()) == 1;
    total = len;
    ok = ok && EVP_DecryptFinal_ex(ctx, reinterpret_cast<unsigned char *>(out.data()) + total, &len) == 1;
    total += len;
    EVP_CIPHER_CTX_free(ctx);
    if (ok)
        *plain = QString::fromUtf8(out.constData(), total);
    return ok;
}

QByteArray keyringPassword(const QString &application) {
    QProcess process;
    process.start("secret-tool", {"lookup", "application", application});
    if (!process.waitForStarted(2000) || !process.waitForFinished(30000) || process.exitCode() != 0)
        return {};
    return process.readAllStandardOutput();
}

QList<QStringList> parseCsv(const QString &text) {
    QList<QStringList> rows;
    QStringList row;
    QString field;
    bool quoted = false;
    for (int i = 0; i < text.size(); ++i) {
        const QChar c = text[i];
        if (quoted) {
            if (c == '"' && i + 1 < text.size() && text[i + 1] == '"') {
                field += '"';
                ++i;
            } else if (c == '"') {
                quoted = false;
            } else {
                field += c;
            }
        } else if (c == '"') {
            quoted = true;
        } else if (c == ',') {
            row.append(field);
            field.clear();
        } else if (c == '\n' || c == '\r') {
            if (c == '\r' && i + 1 < text.size() && text[i + 1] == '\n')
                ++i;
            row.append(field);
            field.clear();
            if (!(row.size() == 1 && row.first().isEmpty()))
                rows.append(row);
            row.clear();
        } else {
            field += c;
        }
    }
    if (!field.isEmpty() || !row.isEmpty()) {
        row.append(field);
        rows.append(row);
    }
    return rows;
}

int columnOf(const QStringList &header, const QStringList &names) {
    for (const QString &name : names) {
        const int index = header.indexOf(name);
        if (index >= 0)
            return index;
    }
    return -1;
}

QString count(int n, const QString &singular, const QString &plural) {
    return QLocale(QLocale::English).toString(n) + " " + (n == 1 ? singular : plural);
}

}  // namespace

Importer::Importer(Backend *backend, Vault *vault, QObject *parent) : QObject(parent), m_backend(backend), m_vault(vault) {}

QVariantList Importer::sources() const {
    QVariantList result;
    for (const BrowserKind &kind : kinds()) {
        const QString root = QDir::homePath() + "/" + kind.root;
        if (!QFileInfo(root).isDir())
            continue;
        const QList<Profile> profiles = kind.firefox ? firefoxProfiles(root) : chromiumProfiles(root);
        for (const Profile &profile : profiles) {
            const QString label = profiles.size() > 1 || profile.name != "Default" ? kind.name + " (" + profile.name + ")" : kind.name;
            result.append(QVariantMap{{"id", kind.key + "|" + profile.dir}, {"label", label}});
        }
    }
    return result;
}

QVariantMap Importer::importSource(const QString &id) {
    const int bar = id.indexOf('|');
    const QString key = id.left(bar);
    const QString dir = id.mid(bar + 1);
    const BrowserKind *kind = nullptr;
    for (const BrowserKind &candidate : kinds())
        if (candidate.key == key)
            kind = &candidate;
    if (!kind || bar < 0 || !QFileInfo(dir).isDir())
        return {{"ok", false}, {"message", "That browser profile is no longer there."}};

    QVariantList bookmarks;
    QVariantList history;
    QList<Vault::Login> logins;
    int undecryptable = 0;

    if (kind->firefox) {
        for (const auto &row : readRows(dir + "/places.sqlite",
                 "SELECT p.url, b.title FROM moz_bookmarks b JOIN moz_places p ON p.id = b.fk "
                 "WHERE b.type = 1 AND p.url LIKE 'http%'", 2))
            bookmarks.append(QVariantMap{{"url", row[0].toString()}, {"title", row[1].toString()}});
        for (const auto &row : readRows(dir + "/places.sqlite",
                 QString("SELECT url, title, last_visit_date FROM moz_places WHERE last_visit_date IS NOT NULL "
                         "AND url LIKE 'http%' ORDER BY last_visit_date DESC LIMIT %1").arg(maxHistory), 3))
            history.append(QVariantMap{{"url", row[0].toString()}, {"title", row[1].toString()},
                                       {"time", row[2].toLongLong() / 1000000}});
    } else {
        QFile file(dir + "/Bookmarks");
        if (file.open(QIODevice::ReadOnly)) {
            const QJsonObject roots = QJsonDocument::fromJson(file.readAll()).object().value("roots").toObject();
            for (const auto &root : roots)
                if (root.isObject())
                    collectChromiumBookmarks(root.toObject(), &bookmarks);
        }
        // Chromium counts microseconds from 1601-01-01.
        for (const auto &row : readRows(dir + "/History",
                 QString("SELECT url, title, last_visit_time FROM urls WHERE last_visit_time > 0 "
                         "AND (url LIKE 'http://%' OR url LIKE 'https://%') ORDER BY last_visit_time DESC LIMIT %1").arg(maxHistory), 3))
            history.append(QVariantMap{{"url", row[0].toString()}, {"title", row[1].toString()},
                                       {"time", row[2].toLongLong() / 1000000 - 11644473600LL}});

        const auto rows = readRows(dir + "/Login Data",
            "SELECT origin_url, username_value, password_value FROM logins "
            "WHERE blacklisted_by_user = 0 AND length(password_value) > 0", 3);
        QByteArray keyringKey;
        bool keyringTried = false;
        for (const auto &row : rows) {
            const QByteArray blob = row[2].toByteArray();
            const bool v11 = blob.startsWith("v11");
            if (!blob.startsWith("v10") && !v11) {
                ++undecryptable;
                continue;
            }
            if (v11 && !keyringTried) {
                keyringKey = keyringPassword(kind->keyringApp);
                keyringTried = true;
            }
            if (v11 && keyringKey.isEmpty()) {
                ++undecryptable;
                continue;
            }
            QString password;
            if (decryptChromium(pbkdf2(v11 ? keyringKey : QByteArray("peanuts")), blob.mid(3), &password))
                logins.append({row[0].toString(), row[1].toString(), password});
            else
                ++undecryptable;
        }
    }

    const QString from = kind->name;
    const int bookmarkCount = m_backend->addImportedBookmarks("Imported from " + from, bookmarks);
    const int historyCount = m_backend->addImportedHistory(history);
    const int passwordCount = m_vault->addImported(logins);

    QStringList parts{count(bookmarkCount, "bookmark", "bookmarks"), count(historyCount, "history entry", "history entries")};
    if (!logins.isEmpty() || undecryptable > 0)
        parts.append(count(passwordCount, "password", "passwords"));
    QString message = "Imported from " + from + ": " + parts.join(", ") + ".";
    if (!logins.isEmpty() && !m_vault->available())
        message += " Passwords were skipped: " + m_vault->problem();
    if (undecryptable > 0)
        message += " " + count(undecryptable, "password", "passwords") + " could not be decrypted.";
    if (kind->firefox)
        message += " For passwords, export them from " + from + " as CSV and use Import CSV.";
    return {{"ok", true}, {"message", message}};
}

QVariantMap Importer::importPasswordsCsv(const QString &fileOrUrl) {
    const QString path = fileOrUrl.startsWith("file:") ? QUrl(fileOrUrl).toLocalFile() : fileOrUrl;
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly))
        return {{"ok", false}, {"message", "That file could not be opened."}};
    const QList<QStringList> rows = parseCsv(QString::fromUtf8(file.readAll()));
    if (rows.size() < 2)
        return {{"ok", false}, {"message", "That file has no passwords in it."}};

    QStringList header;
    for (const QString &name : rows.first())
        header.append(name.trimmed().toLower());
    const int url = columnOf(header, {"url", "login_uri", "origin", "website", "web site", "site"});
    const int user = columnOf(header, {"username", "login_username", "user", "login", "email"});
    const int pass = columnOf(header, {"password", "login_password"});
    if (url < 0 || pass < 0)
        return {{"ok", false}, {"message", "That does not look like a passwords export. It needs url and password columns."}};
    if (!m_vault->available())
        return {{"ok", false}, {"message", m_vault->problem()}};

    QList<Vault::Login> logins;
    for (int i = 1; i < rows.size(); ++i) {
        const QStringList &row = rows[i];
        if (row.size() <= std::max(url, pass))
            continue;
        logins.append({row[url].trimmed(), user >= 0 && user < row.size() ? row[user] : QString(), row[pass]});
    }
    const int added = m_vault->addImported(logins);
    return {{"ok", true}, {"message", "Imported " + count(added, "password", "passwords") + " (" + QString::number(logins.size() - added) + " already saved or skipped)."}};
}
