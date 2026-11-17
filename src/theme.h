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
    // Ink derived from the theme so that text and input edges clear WCAG AA (F10).
    Q_PROPERTY(QString mutedText READ mutedText NOTIFY changed)
    Q_PROPERTY(QString linkText READ linkText NOTIFY changed)
    Q_PROPERTY(QString inputBorder READ inputBorder NOTIFY changed)
    Q_PROPERTY(QString red READ red NOTIFY changed)
    Q_PROPERTY(QString green READ green NOTIFY changed)
    Q_PROPERTY(QString yellow READ yellow NOTIFY changed)
    Q_PROPERTY(bool dark READ dark NOTIFY changed)
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
    QString mutedText() const { return m_mutedText; }
    QString linkText() const { return m_linkText; }
    QString inputBorder() const { return m_inputBorder; }
    QString red() const { return m_red; }
    QString green() const { return m_green; }
    QString yellow() const { return m_yellow; }
    bool dark() const { return m_dark; }
    int radius() const { return m_radius; }
    // The one colours.toml this theme was read from.
    QString colorsPath() const;

signals:
    void changed();

private:
    void reload();
    void refreshRounding();
    // Aliases are tried in order; the first key present wins.
    QString readColor(const QString &content, const QStringList &keys, const QString &fallback);

    QString m_directory;
    QString m_accent;
    QString m_background;
    QString m_surface;
    QString m_chrome;
    QString m_text;
    QString m_textStrong;
    QString m_muted;
    QString m_line;
    QString m_mutedText;
    QString m_linkText;
    QString m_inputBorder;
    QString m_red;
    QString m_green;
    QString m_yellow;
    bool m_dark = true;

    QFileSystemWatcher m_watcher;
    QTimer m_debounce;
    QString m_hyprctl;
    int m_radius = 0;
    QProcess m_roundingProcess;
    QTimer m_roundingTimeout;
};
