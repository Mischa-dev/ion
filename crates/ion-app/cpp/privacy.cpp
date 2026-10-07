#include "ion-app/cpp/privacy.h"

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

} // namespace ion
