// Helpers for the `Downloads` bridge that need Qt's QObject API.
#pragma once

#include <QtCore/QVariant>

namespace ion {

// Schedule the QObject held in `value` (a download request passed from QML)
// for deletion. QML cannot call deleteLater() on objects C++ owns.
void deleteLaterObject(const QVariant &value);

} // namespace ion
