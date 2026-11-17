#pragma once

#include <QObject>
#include <QString>
#include <QVariantList>
#include <QVariantMap>

class Backend;
class Vault;

// Brings bookmarks, history and saved logins in from other browsers installed for this user.
// Chromium family (Chrome, Chromium, Brave, Edge, Vivaldi, Opera) and Firefox family
// (Firefox, Zen, LibreWolf, Floorp, Waterfox) are read straight from their profile folders;
// any browser's passwords export (CSV) can be imported too.
//
// Reading another browser's databases copies files and runs SQL, and decrypting Chromium
// passwords can wait up to 30 s on a keyring prompt, so the work runs on a worker thread.
// Only the results are applied here, on the GUI thread.
class Importer : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool importing READ importing NOTIFY importingChanged)

public:
    Importer(Backend *backend, Vault *vault, QObject *parent = nullptr);

    // One entry per profile found: id, label.
    Q_INVOKABLE QVariantList sources() const;
    // Both start a background import and report through importFinished (ok, message).
    Q_INVOKABLE void importSource(const QString &id);
    Q_INVOKABLE void importPasswordsCsv(const QString &fileOrUrl);
    bool importing() const { return m_importing; }

signals:
    void importFinished(const QVariantMap &result);
    void importingChanged();

private:
    void setImporting(bool importing);
    void finish(const QVariantMap &result);

    Backend *m_backend;
    Vault *m_vault;
    bool m_importing = false;
};
