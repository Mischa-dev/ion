#include "ion-app/cpp/mpris.h"
#include "ion-app/src/bridge/media.cxxqt.h"

#if defined(__linux__)

#include <QtCore/QCoreApplication>
#include <QtCore/QElapsedTimer>
#include <QtCore/QStringList>
#include <QtCore/QVariantMap>
#include <QtDBus/QDBusConnection>
#include <QtDBus/QDBusMessage>
#include <QtDBus/QDBusObjectPath>
#include <QtDBus/QDBusVariant>
#include <QtDBus/QDBusVirtualObject>

namespace ion {

namespace {

// One name per process, as the MPRIS spec suggests for several instances:
// a private window runs in a process of its own.
QString service() {
    return QStringLiteral("org.mpris.MediaPlayer2.ion.instance%1")
        .arg(QCoreApplication::applicationPid());
}
const QString kPath = QStringLiteral("/org/mpris/MediaPlayer2");
const QString kRoot = QStringLiteral("org.mpris.MediaPlayer2");
const QString kPlayer = QStringLiteral("org.mpris.MediaPlayer2.Player");
const QString kProperties = QStringLiteral("org.freedesktop.DBus.Properties");

// The object at /org/mpris/MediaPlayer2, answering calls by hand so no moc
// step is needed.
class Player : public QDBusVirtualObject
{
public:
    MprisTrack track;
    quint64 trackNumber = 0;
    // Time since `track` was last reported, to tell seeks from playback.
    QElapsedTimer sinceUpdate;

    QVariantMap rootProperties() const
    {
        return {
            { QStringLiteral("CanQuit"), false },
            { QStringLiteral("CanRaise"), true },
            { QStringLiteral("HasTrackList"), false },
            { QStringLiteral("Identity"), QStringLiteral("Ion") },
            { QStringLiteral("DesktopEntry"), QStringLiteral("dev.ion.Ion") },
            { QStringLiteral("SupportedUriSchemes"), QStringList() },
            { QStringLiteral("SupportedMimeTypes"), QStringList() },
        };
    }

    QString trackPath() const { return QStringLiteral("/dev/ion/Ion/track/%1").arg(trackNumber); }

    QVariantMap metadata() const
    {
        QVariantMap map;
        map.insert(QStringLiteral("mpris:trackid"), QVariant::fromValue(QDBusObjectPath(trackPath())));
        if (!track.title.isEmpty())
            map.insert(QStringLiteral("xesam:title"), track.title);
        if (!track.artist.isEmpty())
            map.insert(QStringLiteral("xesam:artist"), QStringList { track.artist });
        if (!track.album.isEmpty())
            map.insert(QStringLiteral("xesam:album"), track.album);
        if (!track.artUrl.isEmpty())
            map.insert(QStringLiteral("mpris:artUrl"), track.artUrl);
        if (!track.url.isEmpty())
            map.insert(QStringLiteral("xesam:url"), track.url);
        if (track.lengthUs > 0)
            map.insert(QStringLiteral("mpris:length"), track.lengthUs);
        return map;
    }

    // Pages report every few seconds; in between, playing moves on.
    qint64 positionNow() const
    {
        qint64 position = track.positionUs;
        if (track.playing && sinceUpdate.isValid())
            position += sinceUpdate.elapsed() * 1000;
        return track.lengthUs > 0 ? qMin(position, track.lengthUs) : position;
    }

    QVariantMap playerProperties() const
    {
        return {
            { QStringLiteral("PlaybackStatus"),
              track.playing ? QStringLiteral("Playing") : QStringLiteral("Paused") },
            { QStringLiteral("Rate"), 1.0 },
            { QStringLiteral("MinimumRate"), 1.0 },
            { QStringLiteral("MaximumRate"), 1.0 },
            { QStringLiteral("Volume"), 1.0 },
            { QStringLiteral("Metadata"), metadata() },
            { QStringLiteral("Position"), positionNow() },
            { QStringLiteral("CanGoNext"), track.canNext },
            { QStringLiteral("CanGoPrevious"), track.canPrevious },
            { QStringLiteral("CanPlay"), true },
            { QStringLiteral("CanPause"), true },
            { QStringLiteral("CanSeek"), track.lengthUs > 0 },
            { QStringLiteral("CanControl"), true },
        };
    }

    QString introspect(const QString &path) const override
    {
        if (path != kPath)
            return {};
        return QStringLiteral(
            "<interface name=\"org.mpris.MediaPlayer2\">"
            "<method name=\"Raise\"/><method name=\"Quit\"/>"
            "<property name=\"CanQuit\" type=\"b\" access=\"read\"/>"
            "<property name=\"CanRaise\" type=\"b\" access=\"read\"/>"
            "<property name=\"HasTrackList\" type=\"b\" access=\"read\"/>"
            "<property name=\"Identity\" type=\"s\" access=\"read\"/>"
            "<property name=\"DesktopEntry\" type=\"s\" access=\"read\"/>"
            "<property name=\"SupportedUriSchemes\" type=\"as\" access=\"read\"/>"
            "<property name=\"SupportedMimeTypes\" type=\"as\" access=\"read\"/>"
            "</interface>"
            "<interface name=\"org.mpris.MediaPlayer2.Player\">"
            "<method name=\"Next\"/><method name=\"Previous\"/><method name=\"Pause\"/>"
            "<method name=\"PlayPause\"/><method name=\"Stop\"/><method name=\"Play\"/>"
            "<method name=\"Seek\"><arg name=\"Offset\" type=\"x\" direction=\"in\"/></method>"
            "<method name=\"SetPosition\"><arg name=\"TrackId\" type=\"o\" direction=\"in\"/>"
            "<arg name=\"Position\" type=\"x\" direction=\"in\"/></method>"
            "<method name=\"OpenUri\"><arg name=\"Uri\" type=\"s\" direction=\"in\"/></method>"
            "<signal name=\"Seeked\"><arg name=\"Position\" type=\"x\"/></signal>"
            "<property name=\"PlaybackStatus\" type=\"s\" access=\"read\"/>"
            "<property name=\"Rate\" type=\"d\" access=\"readwrite\"/>"
            "<property name=\"Metadata\" type=\"a{sv}\" access=\"read\"/>"
            "<property name=\"Volume\" type=\"d\" access=\"readwrite\"/>"
            "<property name=\"Position\" type=\"x\" access=\"read\"/>"
            "<property name=\"MinimumRate\" type=\"d\" access=\"read\"/>"
            "<property name=\"MaximumRate\" type=\"d\" access=\"read\"/>"
            "<property name=\"CanGoNext\" type=\"b\" access=\"read\"/>"
            "<property name=\"CanGoPrevious\" type=\"b\" access=\"read\"/>"
            "<property name=\"CanPlay\" type=\"b\" access=\"read\"/>"
            "<property name=\"CanPause\" type=\"b\" access=\"read\"/>"
            "<property name=\"CanSeek\" type=\"b\" access=\"read\"/>"
            "<property name=\"CanControl\" type=\"b\" access=\"read\"/>"
            "</interface>");
    }

    bool handleMessage(const QDBusMessage &message, const QDBusConnection &connection) override
    {
        const QString iface = message.interface();
        const QString member = message.member();
        const QVariantList args = message.arguments();
        if (iface == kProperties) {
            const QString target = args.value(0).toString();
            const QVariantMap props = target == kRoot ? rootProperties()
                : target == kPlayer                   ? playerProperties()
                                                      : QVariantMap();
            if (member == QLatin1String("Get")) {
                const QString name = args.value(1).toString();
                if (!props.contains(name)) {
                    connection.send(message.createErrorReply(
                        QStringLiteral("org.freedesktop.DBus.Error.InvalidArgs"),
                        QStringLiteral("No such property")));
                    return true;
                }
                connection.send(message.createReply(QVariant::fromValue(QDBusVariant(props.value(name)))));
            } else if (member == QLatin1String("GetAll")) {
                connection.send(message.createReply(QVariant::fromValue(props)));
            } else {
                // Set: rate and volume stay as the page has them.
                connection.send(message.createReply());
            }
            return true;
        }
        QString action;
        double value = 0;
        if (iface == kRoot || iface.isEmpty()) {
            // CanQuit is false: a media widget shouldn't close the browser.
            if (member == QLatin1String("Raise"))
                action = QStringLiteral("raise");
            else if (member == QLatin1String("Quit")) {
                connection.send(message.createReply());
                return true;
            }
        }
        if (action.isEmpty() && (iface == kPlayer || iface.isEmpty())) {
            if (member == QLatin1String("Play"))
                action = QStringLiteral("play");
            else if (member == QLatin1String("Pause"))
                action = QStringLiteral("pause");
            else if (member == QLatin1String("PlayPause"))
                action = QStringLiteral("playpause");
            else if (member == QLatin1String("Stop"))
                action = QStringLiteral("stop");
            else if (member == QLatin1String("Next"))
                action = QStringLiteral("next");
            else if (member == QLatin1String("Previous"))
                action = QStringLiteral("previous");
            else if (member == QLatin1String("Seek")) {
                action = QStringLiteral("seekby");
                value = args.value(0).toLongLong() / 1e6;
            } else if (member == QLatin1String("SetPosition")) {
                // A request for a track no longer showing does nothing.
                if (args.value(0).value<QDBusObjectPath>().path() != trackPath()) {
                    connection.send(message.createReply());
                    return true;
                }
                action = QStringLiteral("seekto");
                value = args.value(1).toLongLong() / 1e6;
            } else if (member == QLatin1String("OpenUri")) {
                connection.send(message.createReply());
                return true;
            }
        }
        if (action.isEmpty())
            return false;
        connection.send(message.createReply());
        mprisAction(action, value);
        return true;
    }
};

Player *player()
{
    static Player *instance = [] {
        auto *p = new Player;
        p->setParent(QCoreApplication::instance());
        return p;
    }();
    return instance;
}

bool registered = false;
bool objectRegistered = false;

void propertiesChanged(const QString &iface, const QVariantMap &changed)
{
    if (changed.isEmpty())
        return;
    QDBusMessage signal = QDBusMessage::createSignal(kPath, kProperties,
                                                     QStringLiteral("PropertiesChanged"));
    signal << iface << QVariant::fromValue(changed) << QStringList();
    QDBusConnection::sessionBus().send(signal);
}

} // namespace

void mprisUpdate(bool active, const QString &title, const QString &artist, const QString &album,
                 const QString &artUrl, const QString &url, std::int64_t lengthUs, std::int64_t positionUs,
                 bool playing, bool canPrevious, bool canNext)
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    Player *p = player();
    if (!active) {
        if (registered)
            bus.unregisterService(service());
        registered = false;
        return;
    }
    if (!objectRegistered)
        objectRegistered = bus.registerVirtualObject(kPath, p);
    const MprisTrack next { title, artist, album, artUrl, url, lengthUs, positionUs,
                            playing, canPrevious, canNext };
    const bool newTrack = next.title != p->track.title || next.url != p->track.url;
    const QVariantMap before = p->playerProperties();
    if (newTrack)
        p->trackNumber++;
    // A jump playback alone can't explain is a seek, which MPRIS signals.
    const qint64 elapsedUs = p->sinceUpdate.isValid() ? p->sinceUpdate.elapsed() * 1000 : 0;
    const qint64 expected = p->track.positionUs + (p->track.playing ? elapsedUs : 0);
    const bool seeked = !newTrack && qAbs(next.positionUs - expected) > 2'000'000;
    p->track = next;
    p->sinceUpdate.start();
    if (!registered) {
        registered = bus.registerService(service());
        return;
    }
    QVariantMap changed;
    const QVariantMap after = p->playerProperties();
    for (auto it = after.cbegin(); it != after.cend(); ++it) {
        if (it.key() != QLatin1String("Position") && before.value(it.key()) != it.value())
            changed.insert(it.key(), it.value());
    }
    propertiesChanged(kPlayer, changed);
    if (seeked) {
        QDBusMessage signal = QDBusMessage::createSignal(kPath, kPlayer, QStringLiteral("Seeked"));
        signal << next.positionUs;
        bus.send(signal);
    }
}

} // namespace ion

#else

namespace ion {

void mprisUpdate(bool, const QString &, const QString &, const QString &, const QString &,
                 const QString &, std::int64_t, std::int64_t, bool, bool, bool)
{
}

} // namespace ion

#endif
