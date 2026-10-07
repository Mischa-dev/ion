// Cookie controls for the privacy bridge (src/bridge/privacy.rs).
#pragma once

#include <QtCore/QObject>

namespace ion {

// Refuse (or stop refusing) third-party cookies on a QML WebEngineProfile.
// False if `profile` is not one.
bool setThirdPartyCookiesBlocked(QObject *profile, bool blocked);

// Delete every cookie in a QML WebEngineProfile. False if it is not one.
bool deleteAllCookies(QObject *profile);

} // namespace ion
