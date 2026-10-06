// Helpers for the adblock bridge (src/bridge/adblock.rs), whose `Adblock`
// object is a QWebEngineUrlRequestInterceptor implemented in Rust.
#pragma once

#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtWebEngineCore/QWebEngineUrlRequestInfo>
#include <QtWebEngineCore/QWebEngineUrlRequestInterceptor>

namespace ion {

// Install `interceptor` on a QML WebEngineProfile. False if `profile` is not one.
bool attachUrlRequestInterceptor(QObject *profile, QWebEngineUrlRequestInterceptor *interceptor);

QString requestUrl(const QWebEngineUrlRequestInfo &info);
QString requestFirstPartyUrl(const QWebEngineUrlRequestInfo &info);
QString requestMethod(const QWebEngineUrlRequestInfo &info);
int requestResourceType(const QWebEngineUrlRequestInfo &info);
void blockRequest(QWebEngineUrlRequestInfo &info);

} // namespace ion
