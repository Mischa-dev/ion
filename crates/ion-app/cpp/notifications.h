// Desktop notifications through org.freedesktop.Notifications on Linux, for
// the `DesktopNotifications` bridge. Elsewhere nothing is available and Ion
// shows its own toasts.
#pragma once

#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtCore/QVariant>

namespace ion {

// Whether a notification server is running (or can be started) on the
// session bus.
bool desktopNotificationsAvailable();

// Show a notification and return its id, or 0 if it couldn't be shown.
// `icon` is a QImage in a QVariant (WebEngineNotification.icon), or empty.
// `replacesId` replaces an earlier notification in place (0 for a new one).
quint32 showDesktopNotification(const QString &title, const QString &body, const QString &source,
                                const QVariant &icon, quint32 replacesId);

// Take a notification down, as when the page closes it.
void closeDesktopNotification(quint32 id);

// Receives the notification server's signals and forwards them to Rust.
class DesktopNotificationWatcher : public QObject
{
    Q_OBJECT
public:
    using QObject::QObject;

public Q_SLOTS:
    void onActionInvoked(quint32 id, const QString &action);
    void onActivationToken(quint32 id, const QString &token);
    void onClosed(quint32 id, quint32 reason);

private:
    QString m_token;
};

} // namespace ion
