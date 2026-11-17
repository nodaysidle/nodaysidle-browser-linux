#include "vault.h"

#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QRegularExpression>
#include <QSaveFile>
#include <QTimer>
#include <QUrl>
#include <algorithm>
#include <openssl/evp.h>
#include <openssl/rand.h>

namespace {

const QByteArray magic = "NDV1";
constexpr int ivSize = 12;
constexpr int tagSize = 16;

// A password filled into a plain-http page travels in clear text to anyone on the path,
// so only loopback is allowed there. Everything else needs https.
bool passwordFillAllowed(const QString &origin) {
    const int separator = origin.indexOf("://");
    const QString scheme = separator > 0 ? origin.left(separator) : QString();
    if (scheme == "https")
        return true;
    if (scheme != "http")
        return false;
    static const QRegularExpression loopback(R"(^http://(localhost|127\.0\.0\.1|\[::1\])(:\d+)?$)");
    return loopback.match(origin).hasMatch();
}

QByteArray randomBytes(int count) {
    QByteArray bytes(count, 0);
    // A zero key, IV or nonce would silently break the vault and the login message check.
    if (RAND_bytes(reinterpret_cast<unsigned char *>(bytes.data()), count) != 1)
        qFatal("OpenSSL could not produce random bytes");
    return bytes;
}

// File layout: magic, 12-byte IV, 16-byte GCM tag, ciphertext. The magic is authenticated too.
QByteArray seal(const QByteArray &key, const QByteArray &plain) {
    const QByteArray iv = randomBytes(ivSize);
    QByteArray cipher(plain.size(), 0);
    QByteArray tag(tagSize, 0);
    int len = 0;
    EVP_CIPHER_CTX *ctx = EVP_CIPHER_CTX_new();
    bool ok = EVP_EncryptInit_ex(ctx, EVP_aes_256_gcm(), nullptr, reinterpret_cast<const unsigned char *>(key.constData()),
                                 reinterpret_cast<const unsigned char *>(iv.constData())) == 1
        && EVP_EncryptUpdate(ctx, nullptr, &len, reinterpret_cast<const unsigned char *>(magic.constData()), magic.size()) == 1
        && EVP_EncryptUpdate(ctx, reinterpret_cast<unsigned char *>(cipher.data()), &len,
                             reinterpret_cast<const unsigned char *>(plain.constData()), plain.size()) == 1
        && EVP_EncryptFinal_ex(ctx, reinterpret_cast<unsigned char *>(cipher.data()) + len, &len) == 1
        && EVP_CIPHER_CTX_ctrl(ctx, EVP_CTRL_GCM_GET_TAG, tagSize, tag.data()) == 1;
    EVP_CIPHER_CTX_free(ctx);
    return ok ? magic + iv + tag + cipher : QByteArray();
}

bool unseal(const QByteArray &key, const QByteArray &blob, QByteArray *plain) {
    if (blob.size() < magic.size() + ivSize + tagSize || !blob.startsWith(magic))
        return false;
    const QByteArray iv = blob.mid(magic.size(), ivSize);
    QByteArray tag = blob.mid(magic.size() + ivSize, tagSize);
    const QByteArray cipher = blob.mid(magic.size() + ivSize + tagSize);
    plain->resize(cipher.size());
    int len = 0;
    EVP_CIPHER_CTX *ctx = EVP_CIPHER_CTX_new();
    bool ok = EVP_DecryptInit_ex(ctx, EVP_aes_256_gcm(), nullptr, reinterpret_cast<const unsigned char *>(key.constData()),
                                 reinterpret_cast<const unsigned char *>(iv.constData())) == 1
        && EVP_DecryptUpdate(ctx, nullptr, &len, reinterpret_cast<const unsigned char *>(magic.constData()), magic.size()) == 1
        && EVP_DecryptUpdate(ctx, reinterpret_cast<unsigned char *>(plain->data()), &len,
                             reinterpret_cast<const unsigned char *>(cipher.constData()), cipher.size()) == 1
        && EVP_CIPHER_CTX_ctrl(ctx, EVP_CTRL_GCM_SET_TAG, tagSize, tag.data()) == 1
        && EVP_DecryptFinal_ex(ctx, reinterpret_cast<unsigned char *>(plain->data()) + len, &len) == 1;
    EVP_CIPHER_CTX_free(ctx);
    return ok;
}

}  // namespace

Vault::Vault(const QString &path, const QString &keyKind, QObject *parent)
    : QObject(parent), m_path(path), m_keyKind(keyKind), m_nonce(QString::fromLatin1(randomBytes(16).toHex())) {
    // The keyring can sit locked behind its own prompt for up to 30 s, so it is opened
    // from the event loop: the window is on screen before secret-tool is even started.
    m_problem = "Unlocking the keyring…";
    QTimer::singleShot(0, this, &Vault::unlock);
}

void Vault::unlock() {
    const QStringList attributes{"service", "nodaysidle-browser", "kind", m_keyKind};
    auto *lookup = new QProcess(this);
    connect(lookup, &QProcess::errorOccurred, this, [this, lookup](QProcess::ProcessError error) {
        if (error != QProcess::FailedToStart)
            return;
        lookup->deleteLater();
        m_problem = "Saved passwords need the secret-tool command (package libsecret) and a keyring such as gnome-keyring.";
        emit changed();
    });
    connect(lookup, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this,
            [this, lookup, attributes](int code, QProcess::ExitStatus status) {
        const QByteArray out = lookup->readAllStandardOutput();
        const QString err = QString::fromUtf8(lookup->readAllStandardError()).trimmed();
        lookup->deleteLater();
        if (status != QProcess::NormalExit) {
            m_problem = "The keyring did not answer, so saved passwords are off. Unlock it and restart.";
            emit changed();
            return;
        }
        const QByteArray stored = QByteArray::fromHex(out.trimmed());
        if (code == 0 && stored.size() == 32) {
            m_key = stored;
            finishUnlock();
            return;
        }
        if (code != 0 && !err.isEmpty()) {
            m_problem = "The keyring is not reachable (" + err + "), so saved passwords are off.";
            emit changed();
            return;
        }
        if (QFileInfo::exists(m_path)) {
            m_problem = "The vault key is missing from the keyring, so the saved passwords cannot be opened.";
            emit changed();
            return;
        }
        createKey(attributes);
    });
    lookup->start("secret-tool", QStringList{"lookup"} + attributes);
}

void Vault::createKey(const QStringList &attributes) {
    const QByteArray fresh = randomBytes(32);
    auto *store = new QProcess(this);
    connect(store, &QProcess::errorOccurred, this, [this, store](QProcess::ProcessError error) {
        if (error != QProcess::FailedToStart)
            return;
        store->deleteLater();
        m_problem = "Saved passwords need the secret-tool command (package libsecret) and a keyring such as gnome-keyring.";
        emit changed();
    });
    connect(store, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this,
            [this, store, fresh](int code, QProcess::ExitStatus status) {
        const QString err = QString::fromUtf8(store->readAllStandardError()).trimmed();
        store->deleteLater();
        if (status != QProcess::NormalExit || code != 0) {
            m_problem = "The keyring refused to store the vault key" + (err.isEmpty() ? QString() : " (" + err + ")") + ".";
            emit changed();
            return;
        }
        m_key = fresh;
        finishUnlock();
    });
    store->start("secret-tool", QStringList{"store", "--label", "nodaysidle vault key"} + attributes);
    store->write(fresh.toHex());
    store->closeWriteChannel();
}

void Vault::finishUnlock() {
    m_available = load();
    if (m_available)
        m_problem.clear();
    emit changed();
}

QString Vault::originOf(const QString &url) {
    const QUrl parsed(url);
    const QString scheme = parsed.scheme().toLower();
    if ((scheme != "http" && scheme != "https") || parsed.host().isEmpty())
        return QString();
    QString origin = scheme + "://" + parsed.host().toLower();
    const int port = parsed.port();
    if (port > 0 && !(scheme == "http" && port == 80) && !(scheme == "https" && port == 443))
        origin += ":" + QString::number(port);
    return origin;
}

bool Vault::load() {
    QFile file(m_path);
    if (!file.exists())
        return true;
    if (!file.open(QIODevice::ReadOnly)) {
        m_problem = "The password vault file cannot be read.";
        return false;
    }
    QByteArray plain;
    if (!unseal(m_key, file.readAll(), &plain)) {
        m_problem = "The password vault is damaged or was made with another key, so it was left untouched.";
        return false;
    }
    const QJsonObject root = QJsonDocument::fromJson(plain).object();
    for (const auto &value : root.value("logins").toArray()) {
        const QJsonObject o = value.toObject();
        Entry entry;
        entry.id = o.value("id").toInt();
        entry.login = {o.value("origin").toString(), o.value("user").toString(), o.value("pass").toString()};
        entry.used = static_cast<qint64>(o.value("used").toDouble());
        m_logins.append(entry);
    }
    for (const auto &value : root.value("never").toArray())
        m_never.append(value.toString());
    return true;
}

bool Vault::persist() {
    QJsonArray logins;
    for (const Entry &e : m_logins)
        logins.append(QJsonObject{{"id", e.id}, {"origin", e.login.origin}, {"user", e.login.username},
                                  {"pass", e.login.password}, {"used", static_cast<double>(e.used)}});
    const QByteArray plain = QJsonDocument(QJsonObject{{"logins", logins}, {"never", QJsonArray::fromStringList(m_never)}})
                                 .toJson(QJsonDocument::Compact);
    const QByteArray blob = seal(m_key, plain);
    if (blob.isEmpty())
        return false;
    QDir().mkpath(QFileInfo(m_path).absolutePath());
    QSaveFile file(m_path);
    if (!file.open(QIODevice::WriteOnly))
        return false;
    file.write(blob);
    if (!file.commit())
        return false;
    QFile::setPermissions(m_path, QFile::ReadOwner | QFile::WriteOwner);
    return true;
}

int Vault::find(const QString &origin, const QString &username) const {
    for (int i = 0; i < m_logins.size(); ++i)
        if (m_logins[i].login.origin == origin && m_logins[i].login.username == username)
            return i;
    return -1;
}

int Vault::nextId() const {
    int id = 0;
    for (const Entry &e : m_logins)
        id = std::max(id, e.id);
    return id + 1;
}

QString Vault::fillScript(const QString &pageUrl, const QString &typedUsername, bool fillUsernameOnly) {
    const QString origin = originOf(pageUrl);
    if (!m_available || origin.isEmpty())
        return QString();
    // Nothing secret goes into a page QtWebEngine would not call secure.
    if (!fillUsernameOnly && !passwordFillAllowed(origin))
        return QString();
    QList<int> matches;
    for (int i = 0; i < m_logins.size(); ++i)
        if (m_logins[i].login.origin == origin)
            matches.append(i);
    if (matches.isEmpty())
        return QString();

    int chosen = -1;
    if (!typedUsername.isEmpty())
        chosen = find(origin, typedUsername);
    if (chosen < 0 && fillUsernameOnly && matches.size() == 1)
        chosen = matches.first();
    if (chosen < 0 && !fillUsernameOnly)
        chosen = *std::max_element(matches.begin(), matches.end(), [this](int a, int b) {
            return m_logins[a].used != m_logins[b].used ? m_logins[a].used < m_logins[b].used : m_logins[a].id < m_logins[b].id;
        });
    if (chosen < 0)
        return QString();

    Entry &entry = m_logins[chosen];
    if (!fillUsernameOnly) {
        entry.used = QDateTime::currentSecsSinceEpoch();
        persist();
    }
    const QJsonArray args{entry.login.username, fillUsernameOnly ? QJsonValue(QJsonValue::Null) : QJsonValue(entry.login.password)};
    return "__ndFill.apply(null, " + QString::fromUtf8(QJsonDocument(args).toJson(QJsonDocument::Compact)).replace("</", "<\\/") + ")";
}

QString Vault::offerState(const QString &pageUrl, const QString &username, const QString &password) const {
    const QString origin = originOf(pageUrl);
    if (!m_available || origin.isEmpty() || password.isEmpty())
        return "off";
    if (m_never.contains(origin))
        return "never";
    if (username.isEmpty()) {
        for (const Entry &e : m_logins)
            if (e.login.origin == origin && e.login.password == password)
                return "same";
        return "new";
    }
    const int index = find(origin, username);
    if (index < 0)
        return "new";
    return m_logins[index].login.password == password ? "same" : "update";
}

void Vault::save(const QString &pageUrl, const QString &username, const QString &password) {
    const QString origin = originOf(pageUrl);
    if (!m_available || origin.isEmpty() || password.isEmpty())
        return;
    const int index = find(origin, username);
    const qint64 now = QDateTime::currentSecsSinceEpoch();
    if (index >= 0) {
        m_logins[index].login.password = password;
        m_logins[index].used = now;
    } else {
        m_logins.append({nextId(), {origin, username, password}, now});
    }
    if (persist())
        emit changed();
}

void Vault::never(const QString &pageUrl) {
    const QString origin = originOf(pageUrl);
    if (!m_available || origin.isEmpty() || m_never.contains(origin))
        return;
    m_never.append(origin);
    persist();
}

QVariantList Vault::list(const QString &query) const {
    QList<Entry> sorted;
    const QString needle = query.trimmed().toLower();
    for (const Entry &e : m_logins)
        if (needle.isEmpty() || e.login.origin.contains(needle) || e.login.username.toLower().contains(needle))
            sorted.append(e);
    std::sort(sorted.begin(), sorted.end(), [](const Entry &a, const Entry &b) {
        return a.login.origin != b.login.origin ? a.login.origin < b.login.origin : a.login.username < b.login.username;
    });
    QVariantList result;
    for (const Entry &e : sorted)
        result.append(QVariantMap{{"id", e.id}, {"origin", e.login.origin}, {"username", e.login.username}});
    return result;
}

QString Vault::reveal(int id) const {
    for (const Entry &e : m_logins)
        if (e.id == id)
            return e.login.password;
    return QString();
}

void Vault::remove(int id) {
    for (int i = 0; i < m_logins.size(); ++i) {
        if (m_logins[i].id == id) {
            m_logins.removeAt(i);
            if (persist())
                emit changed();
            return;
        }
    }
}

int Vault::addImported(const QList<Login> &logins) {
    if (!m_available)
        return 0;
    int added = 0;
    for (const Login &login : logins) {
        const QString origin = originOf(login.origin);
        if (origin.isEmpty() || login.password.isEmpty() || find(origin, login.username) >= 0)
            continue;
        m_logins.append({nextId(), {origin, login.username, login.password}, 0});
        ++added;
    }
    if (added > 0 && persist())
        emit changed();
    return added;
}
