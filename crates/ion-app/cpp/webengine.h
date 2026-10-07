// Small C++ shims for QtWebEngine APIs that are much easier to call from C++.
#pragma once

namespace ion {

// Must run before the QGuiApplication is constructed.
void initializeWebEngine();

} // namespace ion
