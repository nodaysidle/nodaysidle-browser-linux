#include "vault.h"

#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QSaveFile>
#include <QUrl>
#include <algorithm>
#include <openssl/evp.h>
#include <openssl/rand.h>

namespace {

const QByteArray magic = "NDV1";
constexpr int ivSize = 12;
constexpr int tagSize = 16;

QByteArray randomBytes(int count) {
    QByteArray bytes(count, 0);
    RAND_bytes(reinterpret_cast<unsigned char *>(bytes.data()), count);
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

struct ToolResult {
    bool started = false;
    bool finished = false;
    int code = -1;
    QByteArray out;
    QString err;
};

ToolResult runSecretTool(const QStringList &args, const QByteArray &input) {
    ToolResult result;
    QProcess process;
    process.start("secret-tool", args);
    result.started = process.waitForStarted(2000);
    if (!result.started)
        return result;
    if (!input.isEmpty())
        process.write(input);
    process.closeWriteChannel();
    // A locked keyring shows its own unlock prompt, so give a person time to answer it.
    result.finished = process.waitForFinished(30000);
    if (!result.finished) {
        process.kill();
        return result;
    }
    result.code = process.exitCode();
    result.out = process.readAllStandardOutput();
    result.err = QString::fromUtf8(process.readAllStandardError()).trimmed();
    return result;
}

}  // namespace

Vault::Vault(const QString &path, const QString &keyKind, QObject *parent)
    : QObject(parent), m_path(path), m_keyKind(keyKind), m_nonce(QString::fromLatin1(randomBytes(16).toHex())) {
    const bool fileExists = QFileInfo::exists(m_path);
    if (!openKey(!fileExists))
        return;
    m_available = load();
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

bool Vault::openKey(bool create) {
    const QStringList attributes{"service", "nodaysidle-browser", "kind", m_keyKind};
    const ToolResult found = runSecretTool(QStringList{"lookup"} + attributes, {});
    if (!found.started) {
        m_problem = "Saved passwords need the secret-tool command (package libsecret) and a keyring such as gnome-keyring.";
        return false;
    }
    if (!found.finished) {
        m_problem = "The keyring did not answer, so saved passwords are off. Unlock it and restart.";
        return false;
    }
    if (found.code != 0 && !found.err.isEmpty()) {
        m_problem = "The keyring is not reachable (" + found.err + "), so saved passwords are off.";
        return false;
    }
    const QByteArray stored = QByteArray::fromHex(found.out.trimmed());
    if (found.code == 0 && stored.size() == 32) {
        m_key = stored;
        return true;
    }
    if (!create) {
        m_problem = "The vault key is missing from the keyring, so the saved passwords cannot be opened.";
        return false;
    }
    const QByteArray fresh = randomBytes(32);
    const ToolResult saved = runSecretTool(QStringList{"store", "--label", "nodaysidle vault key"} + attributes, fresh.toHex());
    if (!saved.finished || saved.code != 0) {
        m_problem = "The keyring refused to store the vault key" + (saved.err.isEmpty() ? QString() : " (" + saved.err + ")") + ".";
        return false;
    }
    m_key = fresh;
    return true;
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
