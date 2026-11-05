#pragma once

#include <QByteArray>
#include <QJsonArray>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantList>

// Saved logins, encrypted at rest with AES-256-GCM. The 32-byte key lives in the
// desktop keyring (Secret Service) and never touches the disk; without a keyring
// the vault reports why and stays off rather than storing anything in the clear.
class Vault : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool available READ available NOTIFY changed)
    Q_PROPERTY(QString problem READ problem NOTIFY changed)
    Q_PROPERTY(int count READ count NOTIFY changed)
    Q_PROPERTY(QString nonce READ nonce CONSTANT)

public:
    struct Login {
        QString origin;
        QString username;
        QString password;
    };

    // path: the vault file. keyKind: the value of the keyring "kind" attribute,
    // so tests can use a throwaway key instead of the real one.
    explicit Vault(const QString &path, const QString &keyKind = "vault-key", QObject *parent = nullptr);

    bool available() const { return m_available; }
    QString problem() const { return m_problem; }
    int count() const { return m_logins.size(); }
    QString nonce() const { return m_nonce; }

    // scheme://host[:port] for http(s) URLs, empty for anything else.
    static QString originOf(const QString &url);

    // JavaScript that fills the page's login form, or empty when nothing matches.
    // fillUsernameOnly is for identifier-first pages that have no password field yet.
    Q_INVOKABLE QString fillScript(const QString &pageUrl, const QString &typedUsername, bool fillUsernameOnly);
    // "new", "update", "same", "never" or "off" for a login the page just submitted.
    Q_INVOKABLE QString offerState(const QString &pageUrl, const QString &username, const QString &password) const;
    Q_INVOKABLE void save(const QString &pageUrl, const QString &username, const QString &password);
    Q_INVOKABLE void never(const QString &pageUrl);
    Q_INVOKABLE QVariantList list(const QString &query) const;
    Q_INVOKABLE QString reveal(int id) const;
    Q_INVOKABLE void remove(int id);

    // Adds logins that are not already stored (same origin and username). Returns how many were added.
    int addImported(const QList<Login> &logins);

signals:
    void changed();

private:
    struct Entry {
        int id = 0;
        Login login;
        qint64 used = 0;
    };

    bool openKey(bool create);
    bool load();
    bool persist();
    int find(const QString &origin, const QString &username) const;
    int nextId() const;

    QString m_path;
    QString m_keyKind;
    QByteArray m_key;
    QString m_nonce;
    QString m_problem;
    bool m_available = false;
    QList<Entry> m_logins;
    QStringList m_never;
};
