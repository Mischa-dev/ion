#include "ion-app/cpp/downloads.h"

#include <QtCore/QObject>

namespace ion {

void deleteLaterObject(const QVariant &value)
{
    if (QObject *object = value.value<QObject *>())
        object->deleteLater();
}

} // namespace ion
