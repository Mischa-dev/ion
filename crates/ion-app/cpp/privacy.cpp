#include "ion-app/cpp/privacy.h"

#include <QtCore/QStandardPaths>
#include <QtWebEngineCore/QWebEngineCookieStore>
#include <QtWebEngineQuick/QQuickWebEngineProfile>

namespace ion {

bool setThirdPartyCookiesBlocked(QObject *profile, bool blocked)
{
    auto *quickProfile = qobject_cast<QQuickWebEngineProfile *>(profile);
    if (!quickProfile)
        return false;
    // The filter runs on Chromium's IO thread, so it captures nothing mutable.
    if (blocked)
        quickProfile->cookieStore()->setCookieFilter(
                [](const QWebEngineCookieStore::FilterRequest &request) { return !request.thirdParty; });
    else
        quickProfile->cookieStore()->setCookieFilter(nullptr);
    return true;
}

bool deleteAllCookies(QObject *profile)
{
    auto *quickProfile = qobject_cast<QQuickWebEngineProfile *>(profile);
    if (!quickProfile)
        return false;
    quickProfile->cookieStore()->deleteAllCookies();
    return true;
}

QString profileStoragePath(const QString &storageName)
{
    return QStandardPaths::writableLocation(QStandardPaths::AppDataLocation)
            + QStringLiteral("/QtWebEngine/") + storageName;
}

} // namespace ion
