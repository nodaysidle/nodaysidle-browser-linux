#pragma once

#include <QFileSystemWatcher>
#include <QObject>
#include <QProcess>
#include <QTimer>
#include <QColor>

class Theme : public QObject {
    Q_OBJECT
    Q_PROPERTY(QString accent READ accent NOTIFY changed)
    Q_PROPERTY(QString accentForeground READ accentForeground NOTIFY changed)
    Q_PROPERTY(QString background READ background NOTIFY changed)
    Q_PROPERTY(QString surface READ surface NOTIFY changed)
    Q_PROPERTY(QString chrome READ chrome NOTIFY changed)
    Q_PROPERTY(QString text READ text NOTIFY changed)
    Q_PROPERTY(QString textStrong READ textStrong NOTIFY changed)
    Q_PROPERTY(QString muted READ muted NOTIFY changed)
    Q_PROPERTY(QString line READ line NOTIFY changed)
    Q_PROPERTY(QString red READ red NOTIFY changed)
    Q_PROPERTY(QString green READ green NOTIFY changed)
    Q_PROPERTY(QString yellow READ yellow NOTIFY changed)
    Q_PROPERTY(int radius READ radius NOTIFY changed)

public:
    explicit Theme(const QString &currentDirectory = {}, QObject *parent = nullptr, const QString &hyprctl = {});
    ~Theme() override;

    QString accent() const { return m_accent; }
    QString accentForeground() const;
    QString background() const { return m_background; }
    QString surface() const { return m_surface; }
    QString chrome() const { return m_chrome; }
    QString text() const { return m_text; }
    QString textStrong() const { return m_textStrong; }
    QString muted() const { return m_muted; }
    QString line() const { return m_line; }
    QString red() const { return m_red; }
    QString green() const { return m_green; }
    QString yellow() const { return m_yellow; }
    int radius() const { return m_radius; }

signals:
    void changed();

private:
    void reload();
    void refreshRounding();
    QString readColor(const QString &content, const QString &key, const QString &fallback);

    QString m_directory;
    QString m_accent = "#FFD60A";
    QString m_background = "#090A0C";
    QString m_surface = "#14171A";
    QString m_chrome = "#0D0F12";
    QString m_text = "#94A3B8";
    QString m_textStrong = "#F8FAFC";
    QString m_muted = "#64748B";
    QString m_line = "#26292E";
    QString m_red = "#F43F5E";
    QString m_green = "#10B981";
    QString m_yellow = "#FBBF24";

    QFileSystemWatcher m_watcher;
    QTimer m_debounce;
    QString m_hyprctl;
    int m_radius = 0;
    QProcess m_roundingProcess;
    QTimer m_roundingPoll, m_roundingTimeout;
};
