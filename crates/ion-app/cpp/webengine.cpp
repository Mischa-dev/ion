#include "ion-app/cpp/webengine.h"

#include <QtCore/QCoreApplication>
#include <QtWebEngineQuick/QtWebEngineQuick>

namespace ion {

void initializeWebEngine()
{
    QCoreApplication::setAttribute(Qt::AA_ShareOpenGLContexts);
    QtWebEngineQuick::initialize();
}

} // namespace ion
