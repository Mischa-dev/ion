// Helpers for the `ThemeEngine` bridge that reach into QtWebEngine.
#pragma once

#include <cstdint>

namespace ion {

// Tell QtWebEngine which light/dark preference pages get
// (`prefers-color-scheme`): 0 the system's, 1 light, 2 dark. Pages pick it up
// the next time their settings are applied.
void setWebColorScheme(std::int32_t scheme);

} // namespace ion
