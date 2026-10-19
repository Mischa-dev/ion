// Helpers for the pages bridge (src/bridge/pages.rs), whose `Pages` object
// is the `ion:` scheme's QWebEngineUrlSchemeHandler, implemented in Rust.
#pragma once

#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtWebEngineCore/QWebEngineUrlRequestJob>
#include <QtWebEngineCore/QWebEngineUrlSchemeHandler>

namespace ion {

// Declare the `ion:` scheme. Must run before the application object exists.
void registerIonScheme();

// Serve `ion:` on a QML WebEngineProfile with `handler`. False if `profile`
// is not one.
bool installIonSchemeHandler(QObject *profile, QWebEngineUrlSchemeHandler *handler);

QString jobUrl(const QWebEngineUrlRequestJob &job);
// Answer `job` with an HTML document.
void replyHtml(QWebEngineUrlRequestJob &job, const QString &html);
void failJobNotFound(QWebEngineUrlRequestJob &job);

} // namespace ion
