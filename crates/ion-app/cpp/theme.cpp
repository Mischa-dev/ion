#include "ion-app/cpp/theme.h"

#include <QtGui/QGuiApplication>
#include <QtGui/QStyleHints>

namespace ion {

namespace {

// The scheme pages should see, or Unknown to leave them the system's.
Qt::ColorScheme wanted = Qt::ColorScheme::Unknown;

Qt::ColorScheme effective()
{
    return wanted == Qt::ColorScheme::Unknown ? QGuiApplication::styleHints()->colorScheme()
                                              : wanted;
}

} // namespace

// QtWebEngine has no API for this. It copies QStyleHints::colorScheme into
// Chromium's web theme on each colorSchemeChanged signal, and reads that theme
// whenever a page's settings are applied. Emitting the signal with Ion's scheme
// sets the web theme without changing the app's own scheme. QtWebEngine also
// resets the web theme when a tab is created and on real system changes, so
// callers re-assert it then (a new tab's first load, and the queued handler
// below for system changes).
void setWebColorScheme(std::int32_t scheme)
{
    QStyleHints *hints = QGuiApplication::styleHints();
    static const bool watching = [hints] {
        QObject::connect(
            hints, &QStyleHints::colorSchemeChanged, qApp,
            [](Qt::ColorScheme scheme) {
                if (scheme != effective())
                    Q_EMIT QGuiApplication::styleHints()->colorSchemeChanged(effective());
            },
            Qt::QueuedConnection);
        return true;
    }();
    Q_UNUSED(watching);

    wanted = scheme == 2 ? Qt::ColorScheme::Dark
           : scheme == 1 ? Qt::ColorScheme::Light
                         : Qt::ColorScheme::Unknown;
    Q_EMIT hints->colorSchemeChanged(effective());
}

} // namespace ion
