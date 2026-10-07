#include "ion-app/cpp/screenshot.h"

#include <QtGui/QClipboard>
#include <QtGui/QGuiApplication>
#include <QtGui/QImage>
#include <QtGui/QPainter>

#include <cmath>

namespace ion {

bool saveImage(const QVariant &image, const QString &path)
{
    const QImage img = image.value<QImage>();
    return !img.isNull() && img.save(path, "PNG");
}

void copyImage(const QVariant &image)
{
    const QImage img = image.value<QImage>();
    if (!img.isNull())
        QGuiApplication::clipboard()->setImage(img);
}

QVariant pasteTile(const QVariant &canvas, const QVariant &tile, double y, double viewport,
                   double total)
{
    const QImage img = tile.value<QImage>();
    if (img.isNull() || viewport <= 0)
        return canvas;
    // Device pixels per CSS pixel, from the tile.
    const double scale = img.height() / viewport;
    QImage out = canvas.value<QImage>();
    if (out.isNull()) {
        out = QImage(img.width(), int(std::lround(total * scale)),
                     QImage::Format_ARGB32_Premultiplied);
        out.fill(Qt::white);
    }
    // The bottom-aligned last tile overlaps its predecessor with identical
    // pixels, so painting over is fine.
    QPainter painter(&out);
    painter.drawImage(0, int(std::lround(y * scale)), img);
    painter.end();
    return out;
}

} // namespace ion
