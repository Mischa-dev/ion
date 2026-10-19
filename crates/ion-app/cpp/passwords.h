// Clipboard helpers for the `Passwords` bridge (src/bridge/passwords.rs).
#pragma once

#include <QtCore/QString>

namespace ion {

// Put `text` on the clipboard marked as a secret, so clipboard managers
// (KDE's Klipper, many Wayland ones, macOS's) leave it out of their history.
void copySecret(const QString &text);

// Clear the clipboard if it still holds `text`: Ion copied it and nobody has
// copied anything since.
void clearSecret(const QString &text);

} // namespace ion
