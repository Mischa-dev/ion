#include "ion-app/cpp/pages.h"

#include <QtCore/QBuffer>
#include <QtWebEngineCore/QWebEngineUrlScheme>
#include <QtWebEngineQuick/QQuickWebEngineProfile>

namespace ion {

void registerIonScheme()
{
    QWebEngineUrlScheme scheme("ion");
    scheme.setSyntax(QWebEngineUrlScheme::Syntax::Host);
    // Local: web pages can't link to or frame Ion's pages; only the address
    // bar and Ion itself open them.
    scheme.setFlags(QWebEngineUrlScheme::SecureScheme | QWebEngineUrlScheme::LocalScheme
                    | QWebEngineUrlScheme::LocalAccessAllowed);
    QWebEngineUrlScheme::registerScheme(scheme);
}

bool installIonSchemeHandler(QObject *profile, QWebEngineUrlSchemeHandler *handler)
{
    auto *quickProfile = qobject_cast<QQuickWebEngineProfile *>(profile);
    if (!quickProfile)
        return false;
    if (quickProfile->urlSchemeHandler("ion") != handler)
        quickProfile->installUrlSchemeHandler("ion", handler);
    return true;
}

QString jobUrl(const QWebEngineUrlRequestJob &job)
{
    return job.requestUrl().toString(QUrl::FullyEncoded);
}

void replyHtml(QWebEngineUrlRequestJob &job, const QString &html)
{
    // The job owns the buffer and deletes it with itself.
    auto *buffer = new QBuffer(&job);
    buffer->setData(html.toUtf8());
    buffer->open(QIODevice::ReadOnly);
    job.reply("text/html", buffer);
}

void failJobNotFound(QWebEngineUrlRequestJob &job)
{
    job.fail(QWebEngineUrlRequestJob::UrlNotFound);
}

} // namespace ion
