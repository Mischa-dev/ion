#include "ion-app/cpp/platform.h"
#include "ion-app/src/bridge/platform.cxxqt.h"

#include <QtCore/QUrl>
#include <QtGui/QFileOpenEvent>
#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>

#ifdef Q_OS_UNIX
#include <QtCore/QSocketNotifier>
#include <csignal>
#include <sys/socket.h>
#include <unistd.h>
#endif

namespace ion {

namespace {

// macOS delivers clicked links and files opened with Ion as QFileOpenEvents on
// the application object instead of on the command line.
class FileOpenFilter : public QObject
{
public:
    using QObject::QObject;

protected:
    bool eventFilter(QObject* watched, QEvent* event) override
    {
        if (event->type() == QEvent::FileOpen) {
            const QUrl url = static_cast<QFileOpenEvent*>(event)->url();
            openUrlFromSystem(url.isLocalFile() ? url.toLocalFile() : url.toString());
            return true;
        }
        return QObject::eventFilter(watched, event);
    }
};

#ifdef Q_OS_UNIX
int quitSignalSockets[2] = {-1, -1};

void onQuitSignal(int)
{
    // Only async-signal-safe calls here; the notifier below quits.
    const char byte = 1;
    [[maybe_unused]] const auto written = ::write(quitSignalSockets[0], &byte, 1);
}

// SIGTERM (logout, shutdown, `kill`) and SIGINT (Ctrl+C in a terminal) quit
// like closing the window, as in Chrome and Firefox: the session is saved and
// the next start isn't taken for a crash.
void quitOnTerminationSignals()
{
    if (::socketpair(AF_UNIX, SOCK_STREAM, 0, quitSignalSockets) != 0)
        return;
    auto* notifier = new QSocketNotifier(quitSignalSockets[1], QSocketNotifier::Read, qApp);
    QObject::connect(notifier, &QSocketNotifier::activated, qApp, [notifier] {
        notifier->setEnabled(false);
        char byte;
        [[maybe_unused]] const auto read = ::read(quitSignalSockets[1], &byte, 1);
        QCoreApplication::quit();
    });
    struct sigaction action = {};
    action.sa_handler = onQuitSignal;
    sigemptyset(&action.sa_mask);
    action.sa_flags = SA_RESTART;
    ::sigaction(SIGTERM, &action, nullptr);
    ::sigaction(SIGINT, &action, nullptr);
}
#endif

} // namespace

void installPlatformIntegration()
{
    QGuiApplication::setWindowIcon(QIcon(QStringLiteral(":/qt/qml/Ion/icons/dev.ion.Ion.svg")));
    qApp->installEventFilter(new FileOpenFilter(qApp));
#ifdef Q_OS_UNIX
    quitOnTerminationSignals();
#endif
}

void setActivationToken(const QString& token)
{
    if (!token.isEmpty())
        qputenv("XDG_ACTIVATION_TOKEN", token.toUtf8());
}

} // namespace ion
