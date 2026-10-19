// Desktop integration shims: window icon, macOS URL-open events, Wayland
// window activation and quitting on termination signals.
#pragma once

#include <QtCore/QString>

namespace ion {

// Set the window icon, start forwarding macOS "open this URL/file" events to
// Rust, and quit cleanly on SIGTERM/SIGINT. Call once the QGuiApplication
// exists.
void installPlatformIntegration();

// Let the next window activation use this Wayland xdg-activation token, which
// a second launch received from its launcher. Qt reads it on requestActivate().
void setActivationToken(const QString& token);

} // namespace ion
