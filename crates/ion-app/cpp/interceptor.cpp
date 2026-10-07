#include "ion-app/cpp/interceptor.h"

#include <QtWebEngineQuick/QQuickWebEngineProfile>

namespace ion {

bool attachUrlRequestInterceptor(QObject *profile, QWebEngineUrlRequestInterceptor *interceptor)
{
    auto *quickProfile = qobject_cast<QQuickWebEngineProfile *>(profile);
    if (!quickProfile)
        return false;
    quickProfile->setUrlRequestInterceptor(interceptor);
    return true;
}

QString requestUrl(const QWebEngineUrlRequestInfo &info)
{
    return info.requestUrl().toString(QUrl::FullyEncoded);
}

QString requestFirstPartyUrl(const QWebEngineUrlRequestInfo &info)
{
    return info.firstPartyUrl().toString(QUrl::FullyEncoded);
}

QString requestMethod(const QWebEngineUrlRequestInfo &info)
{
    return QString::fromLatin1(info.requestMethod());
}

int requestResourceType(const QWebEngineUrlRequestInfo &info)
{
    return static_cast<int>(info.resourceType());
}

void blockRequest(QWebEngineUrlRequestInfo &info)
{
    info.block(true);
}

void redirectRequest(QWebEngineUrlRequestInfo &info, const QString &url)
{
    info.redirect(QUrl(url));
}

} // namespace ion
