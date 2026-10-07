// Clipboard helper for the screenshots bridge (src/bridge/screenshots.rs).
#pragma once

#include <QtCore/QString>

namespace ion {

// Put the image stored at `path` on the clipboard. False if it can't be read.
bool copyImageFileToClipboard(const QString &path);

} // namespace ion
