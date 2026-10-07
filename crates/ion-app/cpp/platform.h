// Desktop integration shims: window icon, macOS URL-open events and Wayland
// window activation.
#pragma once

#include <QtCore/QString>

namespace ion {

// Set the window icon and start forwarding macOS "open this URL/file" events
// to Rust. Call once the QGuiApplication exists.
void installPlatformIntegration();

// Let the next window activation use this Wayland xdg-activation token, which
// a second launch received from its launcher. Qt reads it on requestActivate().
void setActivationToken(const QString& token);

} // namespace ion
