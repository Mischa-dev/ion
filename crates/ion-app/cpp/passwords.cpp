#include "ion-app/cpp/passwords.h"

#include <QtCore/QMimeData>
#include <QtGui/QClipboard>
#include <QtGui/QGuiApplication>

namespace ion {

void copySecret(const QString &text)
{
    auto *data = new QMimeData;
    data->setText(text);
    // KDE's convention, honoured by Klipper and cliphist-style managers.
    data->setData(QStringLiteral("x-kde-passwordManagerHint"), QByteArrayLiteral("secret"));
    // macOS's (nspasteboard.org): keep it out of clipboard history apps.
    data->setData(QStringLiteral("application/x-nspasteboard-concealed-type"), QByteArray());
    QGuiApplication::clipboard()->setMimeData(data);
}

void clearSecret(const QString &text)
{
    QClipboard *clipboard = QGuiApplication::clipboard();
    if (clipboard->text() == text)
        clipboard->clear();
}

} // namespace ion
