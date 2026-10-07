#include "ion-app/cpp/screenshots.h"

#include <QtGui/QClipboard>
#include <QtGui/QGuiApplication>
#include <QtGui/QImage>

namespace ion {

bool copyImageFileToClipboard(const QString &path)
{
    QImage image(path);
    if (image.isNull())
        return false;
    QGuiApplication::clipboard()->setImage(image);
    return true;
}

} // namespace ion
