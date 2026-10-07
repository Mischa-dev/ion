// Helpers for the `Config` bridge that need Qt QML's C++ API.
#pragma once

#include <QtCore/QVariant>

namespace ion {

// JavaScript arrays and objects reach invokables as a QJSValue wrapped in a
// QVariant; unwrap them to QVariantList / QVariantMap. Other values pass through.
QVariant plainVariant(const QVariant &value);

} // namespace ion
