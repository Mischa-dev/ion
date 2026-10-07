#include "ion-app/cpp/config.h"

#include <QtQml/QJSValue>

namespace ion {

QVariant plainVariant(const QVariant &value)
{
    if (value.metaType() == QMetaType::fromType<QJSValue>())
        return value.value<QJSValue>().toVariant(QJSValue::ConvertJSObjects);
    return value;
}

} // namespace ion
