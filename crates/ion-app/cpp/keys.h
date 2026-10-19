// Key sequence text for the `Keymap` bridge, which needs QKeySequence.
#pragma once

#include <QtCore/QString>

namespace ion {

// The key press `key` with `modifiers` (a QKeyEvent's key() and modifiers())
// as portable key sequence text, "Ctrl+Shift+T". Empty for a press of a
// modifier alone or a key Qt can't name.
QString keySequenceFromEvent(int key, int modifiers);

// Portable key sequence text the way this platform writes it: "⇧⌘T" on
// macOS, "Ctrl+Shift+T" elsewhere.
QString keySequenceNativeText(const QString &portable);

} // namespace ion
