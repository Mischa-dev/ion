// Ion's interface typeface.
#pragma once

namespace ion {

// Register the fonts shipped with Ion (`ION_FONT_DIR`, set by the Nix
// package) and make Inter the interface font when it is available. macOS keeps
// its system font. Must run after the QGuiApplication exists.
void installUiFont();

} // namespace ion
