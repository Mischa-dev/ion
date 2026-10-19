#include "ion-app/cpp/notifications.h"
#include "ion-app/src/bridge/notifications.cxxqt.h"

#include <QtCore/QtGlobal>

#if defined(Q_OS_LINUX)
#include <QtCore/QCoreApplication>
#include <QtCore/QMetaType>
#include <QtDBus/QDBusArgument>
#include <QtDBus/QDBusConnection>
#include <QtDBus/QDBusConnectionInterface>
#include <QtDBus/QDBusInterface>
#include <QtDBus/QDBusMetaType>
#include <QtDBus/QDBusReply>
#include <QtGui/QImage>
#endif

namespace ion {

void DesktopNotificationWatcher::onActionInvoked(quint32 id, const QString &action)
{
    if (action == QLatin1String("default"))
        desktopNotificationClicked(id, m_token);
    m_token.clear();
}

// Sent right before ActionInvoked (spec 1.2), so Wayland lets the window
// come forward.
void DesktopNotificationWatcher::onActivationToken(quint32, const QString &token)
{
    m_token = token;
}

void DesktopNotificationWatcher::onClosed(quint32 id, quint32)
{
    desktopNotificationClosed(id);
}

#if defined(Q_OS_LINUX)

namespace {

const QString kService = QStringLiteral("org.freedesktop.Notifications");
const QString kPath = QStringLiteral("/org/freedesktop/Notifications");
const QString kInterface = QStringLiteral("org.freedesktop.Notifications");

// The `image-data` hint: (iiibiiay) width, height, rowstride, has alpha,
// bits per sample, channels, RGBA bytes.
struct ImageData
{
    QImage image;
};

QDBusArgument &operator<<(QDBusArgument &arg, const ImageData &data)
{
    const QImage image = data.image.convertToFormat(QImage::Format_RGBA8888);
    arg.beginStructure();
    arg << image.width() << image.height() << int(image.bytesPerLine()) << true << 8 << 4
        << QByteArray(reinterpret_cast<const char *>(image.constBits()), image.sizeInBytes());
    arg.endStructure();
    return arg;
}

const QDBusArgument &operator>>(const QDBusArgument &arg, ImageData &)
{
    return arg;
}

QDBusInterface *server()
{
    static QDBusInterface *iface = [] {
        auto *iface = new QDBusInterface(kService, kPath, kInterface, QDBusConnection::sessionBus(),
                                         qApp);
        auto *watcher = new DesktopNotificationWatcher(qApp);
        QDBusConnection bus = QDBusConnection::sessionBus();
        bus.connect(kService, kPath, kInterface, QStringLiteral("ActionInvoked"), watcher,
                    SLOT(onActionInvoked(quint32, QString)));
        bus.connect(kService, kPath, kInterface, QStringLiteral("ActivationToken"), watcher,
                    SLOT(onActivationToken(quint32, QString)));
        bus.connect(kService, kPath, kInterface, QStringLiteral("NotificationClosed"), watcher,
                    SLOT(onClosed(quint32, quint32)));
        return iface;
    }();
    return iface;
}

} // namespace

} // namespace ion

Q_DECLARE_METATYPE(ion::ImageData)

namespace ion {

bool desktopNotificationsAvailable()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected() || !bus.interface())
        return false;
    if (bus.interface()->isServiceRegistered(kService))
        return true;
    const QDBusReply<QStringList> activatable =
        bus.interface()->call(QStringLiteral("ListActivatableNames"));
    return activatable.isValid() && activatable.value().contains(kService);
}

quint32 showDesktopNotification(const QString &title, const QString &body, const QString &source,
                                const QVariant &icon, quint32 replacesId)
{
    static const QMetaType imageType = qDBusRegisterMetaType<ImageData>();
    Q_UNUSED(imageType);

    QVariantMap hints;
    hints.insert(QStringLiteral("desktop-entry"), QStringLiteral("dev.ion.Ion"));
    hints.insert(QStringLiteral("category"), QStringLiteral("im"));
    const QImage image = icon.value<QImage>();
    if (!image.isNull()) {
        const QImage scaled = image.width() > 256 || image.height() > 256
            ? image.scaled(256, 256, Qt::KeepAspectRatio, Qt::SmoothTransformation)
            : image;
        hints.insert(QStringLiteral("image-data"), QVariant::fromValue(ImageData { scaled }));
    }
    // Servers that render a little markup would render the page's too, so
    // escape it for them.
    static const bool markup = [] {
        const QDBusReply<QStringList> caps = server()->call(QStringLiteral("GetCapabilities"));
        return caps.isValid() && caps.value().contains(QStringLiteral("body-markup"));
    }();
    QString text = markup ? body.toHtmlEscaped() : body;
    if (!source.isEmpty())
        text = text.isEmpty() ? source : text + QStringLiteral("\n") + source;

    const QDBusReply<quint32> reply = server()->call(
        QStringLiteral("Notify"), QStringLiteral("Ion"), replacesId, QStringLiteral("dev.ion.Ion"),
        title, text, QStringList { QStringLiteral("default"), QStringLiteral("Open") }, hints,
        -1);
    if (!reply.isValid()) {
        qWarning("ion: desktop notification failed: %s", qPrintable(reply.error().message()));
        return 0;
    }
    return reply.value();
}

void closeDesktopNotification(quint32 id)
{
    if (id != 0)
        server()->asyncCall(QStringLiteral("CloseNotification"), id);
}

#else

bool desktopNotificationsAvailable()
{
    return false;
}

quint32 showDesktopNotification(const QString &, const QString &, const QString &,
                                const QVariant &, quint32)
{
    return 0;
}

void closeDesktopNotification(quint32) { }

#endif

} // namespace ion
