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
class Importer : public QObject {
    Q_OBJECT

public:
    Importer(Backend *backend, Vault *vault, QObject *parent = nullptr);

    // One entry per profile found: id, label.
    Q_INVOKABLE QVariantList sources() const;
    // Returns ok (bool) and message (a sentence for the user).
    Q_INVOKABLE QVariantMap importSource(const QString &id);
    Q_INVOKABLE QVariantMap importPasswordsCsv(const QString &fileOrUrl);

private:
    Backend *m_backend;
    Vault *m_vault;
};
