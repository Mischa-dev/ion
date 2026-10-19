#include "ion-app/cpp/keys.h"

#include <QtGui/QKeySequence>

namespace ion {

QString keySequenceFromEvent(int key, int modifiers)
{
    switch (key) {
    case Qt::Key_unknown:
    case Qt::Key_Control:
    case Qt::Key_Shift:
    case Qt::Key_Alt:
    case Qt::Key_AltGr:
    case Qt::Key_Meta:
    case Qt::Key_Super_L:
    case Qt::Key_Super_R:
    case Qt::Key_Hyper_L:
    case Qt::Key_Hyper_R:
    case Qt::Key_CapsLock:
    case Qt::Key_NumLock:
    case Qt::Key_ScrollLock:
        return QString();
    default:
        break;
    }
    const auto mods = Qt::KeyboardModifiers(modifiers)
        & (Qt::ControlModifier | Qt::AltModifier | Qt::ShiftModifier | Qt::MetaModifier);
    // Shift+Tab arrives as Backtab; keep the spelling people expect.
    if (key == Qt::Key_Backtab)
        key = Qt::Key_Tab;
    const QKeySequence sequence(QKeyCombination(mods, Qt::Key(key)));
    return sequence.toString(QKeySequence::PortableText);
}

QString keySequenceNativeText(const QString &portable)
{
    return QKeySequence::fromString(portable, QKeySequence::PortableText)
        .toString(QKeySequence::NativeText);
}

} // namespace ion
