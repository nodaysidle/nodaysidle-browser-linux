#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantList>
#include <QVariantMap>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QPointer>
#include <QTimer>

class Backend : public QObject {
    Q_OBJECT
    Q_PROPERTY(QVariantList bookmarks READ bookmarks NOTIFY bookmarksChanged)
    Q_PROPERTY(bool hasJevKey READ hasJevKey CONSTANT)
    Q_PROPERTY(bool hasDeepSeekKey READ hasDeepSeekKey CONSTANT)
    Q_PROPERTY(QString homeUrl READ homeUrl CONSTANT)
    Q_PROPERTY(QStringList initialUrls READ initialUrls CONSTANT)
    Q_PROPERTY(QVariantList sessionTabs READ sessionTabs CONSTANT)
    Q_PROPERTY(int sessionIndex READ sessionIndex CONSTANT)

public:
    explicit Backend(QObject *parent = nullptr);
    ~Backend() override = default;

    QVariantList bookmarks() const { return m_bookmarks; }
    bool hasJevKey() const { return !m_jevKey.isEmpty(); }
    bool hasDeepSeekKey() const { return !m_deepseekKey.isEmpty(); }
    QString homeUrl() const;
    QStringList initialUrls() const { return m_initialUrls; }
    void setInitialUrls(const QStringList &urls) { m_initialUrls = urls; }
    QVariantList sessionTabs() const { return m_sessionTabs; }
    int sessionIndex() const { return m_sessionIndex; }

    // The open tabs from the last run, so a quit does not throw them away.
    Q_INVOKABLE void saveSession(const QVariantList &urls, int current);

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
    // Puts the last cleared history or deleted bookmark back (F18).
    Q_INVOKABLE void undoClearHistory();
    Q_INVOKABLE void undoRemoveBookmark();
    void flushHistory();
    // Used by the importer; both skip what is already stored and return how many were added.
    int addImportedBookmarks(const QString &folder, const QVariantList &items);
    int addImportedHistory(const QVariantList &items);

    Q_INVOKABLE QString pdfPath(const QString &title) const;
    Q_INVOKABLE QString readResource(const QString &path) const;
    Q_INVOKABLE QString downloadDir() const;
    // Opens a folder or file with the desktop's handler (xdg-open).
    Q_INVOKABLE void openPath(const QString &path) const;

    // TypeSafe Jev scam check and DeepSeek page assistant.
    Q_INVOKABLE void checkScam(const QString &title, const QString &url, const QString &text);
    Q_INVOKABLE void askPage(const QString &title, const QString &url, const QString &text, const QVariantList &messages);
    Q_INVOKABLE void cancelAsk();

signals:
    void openRequested(const QString &url);
    void notice(const QString &text);
    void bookmarksChanged();
    void assistantError(const QString &error);
    void scamResult(const QString &verdict, int impersonation, int credentials, int pressure, const QString &host);
    void askChunk(const QString &text);
    void askDone();
    void askError(const QString &error);

private:
    void loadKeys();
    void loadBookmarks();
    void loadHistory();
    void loadSession();
    void reindexBookmarks();
    void saveBookmarks();
    QString findKeyInFish(const QString &target);

    QVariantList m_bookmarks;
    QVariantList m_history;
    QVariantList m_historyBackup;
    QVariantMap m_bookmarkBackup;
    bool m_bookmarkBackupValid = false;
    QVariantList m_sessionTabs;
    int m_sessionIndex = 0;
    QTimer m_historySave;
    QStringList m_initialUrls;
    QString m_jevKey;
    QString m_deepseekKey;
    // Lower-cased bookmark URLs, so isBookmarked() is O(1).
    QSet<QString> m_bookmarkUrls;
    QNetworkAccessManager m_nam;
    QPointer<QNetworkReply> m_askReply;
};
