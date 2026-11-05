#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantList>
#include <QVariantMap>
#include <QNetworkAccessManager>
#include <QNetworkReply>

class Backend : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantList bookmarks READ bookmarks NOTIFY bookmarksChanged)
    Q_PROPERTY(bool hasAnthropicKey READ hasAnthropicKey CONSTANT)
    Q_PROPERTY(bool hasJevKey READ hasJevKey CONSTANT)
    Q_PROPERTY(QString homeUrl READ homeUrl CONSTANT)
    Q_PROPERTY(QStringList initialUrls READ initialUrls CONSTANT)

public:
    explicit Backend(QObject *parent = nullptr);
    ~Backend() override = default;

    QVariantList bookmarks() const { return m_bookmarks; }
    bool hasAnthropicKey() const { return !m_anthropicKey.isEmpty(); }
    bool hasJevKey() const { return !m_jevKey.isEmpty(); }
    QString homeUrl() const;
    QStringList initialUrls() const { return m_initialUrls; }
    void setInitialUrls(const QStringList &urls) { m_initialUrls = urls; }

    Q_INVOKABLE QString normalizeUrl(const QString &input);
    Q_INVOKABLE bool isBookmarked(const QString &url);
    Q_INVOKABLE void toggleBookmark(const QString &title, const QString &url);
    Q_INVOKABLE void removeBookmark(const QString &url);
    Q_INVOKABLE void copyText(const QString &text);
    Q_INVOKABLE QString readResource(const QString &path) const;

    // AI Assistant
    Q_INVOKABLE void askAssistant(const QString &title, const QString &url, const QString &text, const QString &prompt);
    Q_INVOKABLE void checkScam(const QString &title, const QString &url, const QString &text);
    Q_INVOKABLE void cancelAssistant();

signals:
    void openRequested(const QString &url);
    void notice(const QString &text);
    void bookmarksChanged();
    void assistantChunk(const QString &text);
    void assistantDone();
    void assistantError(const QString &error);
    void scamResult(const QString &verdict, int impersonation, int credentials, int pressure, const QString &host);

private:
    void loadKeys();
    void loadBookmarks();
    void saveBookmarks();
    QString findKeyInFish(const QString &target);

    QVariantList m_bookmarks;
    QStringList m_initialUrls;
    QString m_anthropicKey;
    QString m_jevKey;
    QNetworkAccessManager m_nam;
    QNetworkReply *m_activeReply = nullptr;
};
