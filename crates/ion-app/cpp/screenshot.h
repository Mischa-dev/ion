// Image helpers for the `Screenshot` bridge: QML hands over the QImage from
// Item.grabToImage() as a QVariant.
#pragma once

#include <QtCore/QString>
#include <QtCore/QVariant>

namespace ion {

// Write the image in `image` to `path` as PNG. False on failure.
bool saveImage(const QVariant &image, const QString &path);

// Put the image in `image` on the clipboard.
void copyImage(const QVariant &image);

// Paint one full-page tile, grabbed at scroll position `y`, onto `canvas`
// and return the result; an empty `canvas` starts a new page-high image.
// `y`, `viewport` and `total` are CSS pixels. The grab result's image only
// lives as long as the grab callback, so each tile is pasted right away.
QVariant pasteTile(const QVariant &canvas, const QVariant &tile, double y, double viewport,
                   double total);

} // namespace ion
