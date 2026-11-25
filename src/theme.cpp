#include "theme.h"

#include <QColor>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QRegularExpression>
#include <QStandardPaths>
#include <algorithm>
#include <cmath>

namespace {

// Only used when a theme file does not name a role at all; everything else is derived.
constexpr const char *fallbackAccent = "#FFD60A";
constexpr const char *fallbackBackground = "#090A0C";
constexpr const char *fallbackForeground = "#F8FAFC";
constexpr const char *fallbackRed = "#F43F5E";
constexpr const char *fallbackGreen = "#10B981";

QString defaultThemeDirectory() {
    const QString state = QDir::homePath() + "/.local/state/omarchy/current";
    if (QFileInfo::exists(state + "/theme/colors.toml"))
        return state;
    // Some installs only keep the symlink source under ~/.config.
    const QString config = QDir::homePath() + "/.config/omarchy/current";
    if (QFileInfo::exists(config + "/theme/colors.toml"))
        return config;
    return state;
}

double channel(double value) {
    return value <= .04045 ? value / 12.92 : std::pow((value + .055) / 1.055, 2.4);
}

double luminance(const QColor &color) {
    return .2126 * channel(color.redF()) + .7152 * channel(color.greenF()) + .0722 * channel(color.blueF());
}

double contrast(const QColor &a, const QColor &b) {
    const double la = luminance(a);
    const double lb = luminance(b);
    return (std::max(la, lb) + .05) / (std::min(la, lb) + .05);
}

// Blend on 8-bit channels: the colour that gets exported as #rrggbb is the one whose
// contrast is measured, so the check and the shipped value cannot drift apart.
QColor mix(const QColor &a, const QColor &b, double t) {
    const auto blend = [t](int from, int to) {
        return std::clamp(qRound(from + (to - from) * t), 0, 255);
    };
    return QColor::fromRgb(blend(a.red(), b.red()), blend(a.green(), b.green()), blend(a.blue(), b.blue()));
}

// Move an ink colour towards black or white until it clears the WCAG ratio against every
// background it is drawn on. Themes are hand-written, so muted and accent are often under
// 4.5:1, and a colour that passes on the panel can still fail on the darker chrome.
QColor ensureContrast(const QColor &color, const QList<QColor> &backgrounds, double ratio) {
    const auto clears = [&backgrounds, ratio](const QColor &candidate) {
        for (const QColor &background : backgrounds)
            if (contrast(candidate, background) < ratio)
                return false;
        return true;
    };
    if (clears(color))
        return color;
    for (const QColor &target : {QColor(Qt::black), QColor(Qt::white)}) {
        for (int step = 1; step <= 20; ++step) {
            const QColor candidate = mix(color, target, step / 20.0);
            if (clears(candidate))
                return candidate;
        }
    }
    return color;
}

}  // namespace

Theme::Theme(const QString &directory, QObject *parent, const QString &hyprctl)
    : QObject(parent), m_directory(directory.isEmpty() ? defaultThemeDirectory() : directory) {
    m_debounce.setSingleShot(true);
    m_debounce.setInterval(80);
    connect(&m_watcher, &QFileSystemWatcher::directoryChanged, this, [this] { m_debounce.start(); });
    connect(&m_watcher, &QFileSystemWatcher::fileChanged, this, [this] { m_debounce.start(); });
    connect(&m_debounce, &QTimer::timeout, this, &Theme::reload);

    m_hyprctl = !hyprctl.isEmpty() ? hyprctl
              : directory.isEmpty() && !qEnvironmentVariableIsEmpty("HYPRLAND_INSTANCE_SIGNATURE")
                  ? QStandardPaths::findExecutable("hyprctl") : QString();
    m_roundingTimeout.setSingleShot(true);
    m_roundingTimeout.setInterval(500);
    connect(&m_roundingTimeout, &QTimer::timeout, &m_roundingProcess, &QProcess::kill);
    connect(&m_roundingProcess, &QProcess::errorOccurred, this, [this] { m_roundingTimeout.stop(); });
    connect(&m_roundingProcess, &QProcess::finished, this, [this](int code, QProcess::ExitStatus status) {
        m_roundingTimeout.stop();
        const auto value = QJsonDocument::fromJson(m_roundingProcess.readAllStandardOutput()).object().value("int");
        if (code != 0 || status != QProcess::NormalExit || !value.isDouble() || value.toInt(-1) < 0)
            return;
        if (value.toInt() != m_radius) {
            m_radius = value.toInt();
            emit changed();
        }
    });
    // The corner radius is read once here and again in reload(), which the theme watcher
    // fires on a theme switch. Polling hyprctl every second cost ~86k processes a day.
    reload();
}

Theme::~Theme() {
    if (m_roundingProcess.state() != QProcess::NotRunning) {
        m_roundingProcess.kill();
        m_roundingProcess.waitForFinished(1000);
    }
}

QString Theme::colorsPath() const {
    // ~/.local/state/omarchy/current keeps the file under theme/; the theme packages
    // under /usr/share/omarchy/themes/<name>/ keep it at the top level.
    const QString nested = m_directory + "/theme/colors.toml";
    if (QFileInfo::exists(nested))
        return nested;
    return m_directory + "/colors.toml";
}

void Theme::refreshRounding() {
    if (m_hyprctl.isEmpty() || m_roundingProcess.state() != QProcess::NotRunning)
        return;
    m_roundingProcess.start(m_hyprctl, {"-j", "getoption", "decoration:rounding"});
    m_roundingTimeout.start();
}

QString Theme::readColor(const QString &content, const QStringList &keys, const QString &fallback) {
    for (const QString &key : keys) {
        const QRegularExpression expr(QString(R"re(^\s*%1\s*=\s*["'](#[0-9a-fA-F]{6})["'])re").arg(key),
                                      QRegularExpression::MultilineOption);
        const auto match = expr.match(content);
        if (match.hasMatch())
            return match.captured(1);
    }
    return fallback;
}

QString Theme::accentForeground() const {
    const QColor c(m_accent);
    return luminance(c) > .179 ? "#000000" : "#FFFFFF";
}

void Theme::reload() {
    refreshRounding();
    const auto paths = m_watcher.files() + m_watcher.directories();
    if (!paths.isEmpty())
        m_watcher.removePaths(paths);

    QString ancestor = m_directory;
    while (!QFileInfo::exists(ancestor) && ancestor != "/")
        ancestor = QFileInfo(ancestor).absolutePath();
    QStringList candidates{ancestor, QFileInfo(ancestor).absolutePath(), m_directory,
                           m_directory + "/theme", colorsPath()};
    candidates.removeDuplicates();
    for (const auto &path : candidates)
        if (QFileInfo::exists(path))
            m_watcher.addPath(path);

    // One fallback table: these values are the starting point whether or not a file exists.
    m_accent = fallbackAccent;
    m_background = fallbackBackground;
    const QString foregroundFallback = fallbackForeground;
    m_red = fallbackRed;
    m_green = fallbackGreen;

    QString foreground = foregroundFallback;
    QString surface, chrome, text, muted, line;

    QFile file(colorsPath());
    if (file.open(QIODevice::ReadOnly)) {
        const QString content = QString::fromUtf8(file.readAll());
        m_accent = readColor(content, {"accent"}, fallbackAccent);
        m_background = readColor(content, {"background", "bg"}, fallbackBackground);
        foreground = readColor(content, {"foreground", "fg", "light_foreground"}, foregroundFallback);
        // Omarchy themes name the same roles differently; prisma-cube uses the *_bg / *_fg set.
        surface = readColor(content, {"lighter_background", "lighter_bg"}, QString());
        chrome = readColor(content, {"dark_background", "dark_bg", "darker_bg"}, QString());
        text = readColor(content, {"light_foreground", "light_fg"}, QString());
        muted = readColor(content, {"muted"}, QString());
        line = readColor(content, {"selection", "selection_background", "line"}, QString());
        m_red = readColor(content, {"red"}, fallbackRed);
        m_green = readColor(content, {"green"}, fallbackGreen);

        const auto modeMatch = QRegularExpression(R"re(^\s*mode\s*=\s*"(\w+)")re",
                                                  QRegularExpression::MultilineOption)
                                   .match(content);
        m_dark = modeMatch.hasMatch() ? modeMatch.captured(1) == "dark" : luminance(QColor(m_background)) < .5;
    } else {
        m_dark = true;
    }

    // Nothing stays a hardcoded slate: derive what the theme did not name from its own pair.
    const QColor bg(m_background);
    const QColor fg(foreground);
    m_surface = surface.isEmpty() ? mix(bg, fg, 0.08).name() : surface;
    m_chrome = chrome.isEmpty() ? mix(bg, fg, 0.04).name() : chrome;
    m_text = text.isEmpty() ? mix(fg, bg, 0.35).name() : text;
    const QString mutedBase = muted.isEmpty() ? mix(fg, bg, 0.55).name() : muted;
    m_line = line.isEmpty() ? mix(bg, fg, 0.16).name() : line;

    // WCAG AA for body text on both panel surfaces, and 3:1 for the input edge.
    const QList<QColor> panels{QColor(m_surface), QColor(m_chrome)};
    const QList<QColor> surfaceOnly{QColor(m_surface)};
    m_textStrong = ensureContrast(QColor(foreground), panels, 4.5).name();
    m_text = ensureContrast(QColor(m_text), panels, 4.5).name();
    m_mutedText = ensureContrast(QColor(mutedBase), panels, 4.5).name();
    m_linkText = ensureContrast(QColor(m_accent), panels, 4.5).name();
    m_inputBorder = ensureContrast(QColor(m_line), surfaceOnly, 3.0).name();
    // The status colours are text too ("Delete", import errors, the scam verdict).
    m_red = ensureContrast(QColor(m_red), surfaceOnly, 4.5).name();
    m_green = ensureContrast(QColor(m_green), surfaceOnly, 4.5).name();

    emit changed();
}
