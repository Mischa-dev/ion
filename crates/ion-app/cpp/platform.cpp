#include "ion-app/cpp/platform.h"
#include "ion-app/src/bridge/platform.cxxqt.h"

#include <QtCore/QUrl>
#include <QtGui/QFileOpenEvent>
#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>

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

} // namespace

void installPlatformIntegration()
{
    QGuiApplication::setWindowIcon(QIcon(QStringLiteral(":/qt/qml/Ion/icons/dev.ion.Ion.svg")));
    qApp->installEventFilter(new FileOpenFilter(qApp));
}

void setActivationToken(const QString& token)
{
    if (!token.isEmpty())
        qputenv("XDG_ACTIVATION_TOKEN", token.toUtf8());
}

} // namespace ion
