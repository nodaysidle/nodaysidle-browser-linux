#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantList>
#include <QVariantMap>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QTimer>

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
    // History
    Q_INVOKABLE void recordVisit(const QString &url, const QString &title);
    Q_INVOKABLE QVariantList searchHistory(const QString &query, int limit);
    Q_INVOKABLE QVariantList suggest(const QString &query, int limit);
    Q_INVOKABLE void clearHistory();
    void flushHistory();
    // Used by the importer; both skip what is already stored and return how many were added.
    int addImportedBookmarks(const QString &folder, const QVariantList &items);
    int addImportedHistory(const QVariantList &items);

    Q_INVOKABLE QString pdfPath(const QString &title) const;
    Q_INVOKABLE QString readResource(const QString &path) const;

    // AI Assistant
    Q_INVOKABLE void askAssistant(const QString &title, const QString &url, const QString &text, const QString &prompt);
    Q_INVOKABLE void checkScam(const QString &title, const QString &url, const QString &text);
    Q_INVOKABLE void cancelAssistant();

signals:
    void historyChanged();
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
    void loadHistory();
    void saveBookmarks();
    QString findKeyInFish(const QString &target);

    QVariantList m_bookmarks;
    QVariantList m_history;
    QTimer m_historySave;
    QStringList m_initialUrls;
    QString m_anthropicKey;
    QString m_jevKey;
    QNetworkAccessManager m_nam;
    QNetworkReply *m_activeReply = nullptr;
};
