{
  lib,
  stdenv,
  mkShell,
  callPackage,
  qt6,
  cargo,
  rustc,
  rustfmt,
  clippy,
  rust-analyzer,
  rustPlatform,
  nixfmt,
  ion,
}:
let
  qt = callPackage ./qt.nix { };
in
mkShell {
  inputsFrom = [ ion ];

  # Debug builds are unoptimized; fortify only produces warnings there.
  hardeningDisable = [ "fortify" ];

  packages = [
    cargo
    rustc
    rustfmt
    clippy
    rust-analyzer
    nixfmt
    qt6.qttools # qmllint, qmlformat, qml
  ];

  env = {
    QT_PLUGIN_PATH = "${qt.env}/${qt6.qtbase.qtPluginPrefix}";
    QML_IMPORT_PATH = "${qt.env}/${qt6.qtbase.qtQmlPrefix}";
    RUST_SRC_PATH = rustPlatform.rustLibSrc;
  }
  // lib.optionalAttrs stdenv.hostPlatform.isLinux {
    QTWEBENGINEPROCESS_PATH = "${qt6.qtwebengine}/libexec/QtWebEngineProcess";
  };

  # qtbase's setup hook points QMAKE at qtbase alone; cxx-qt-build needs the
  # merged prefix so it can find QtQml, QtWebEngine and friends.
  shellHook = ''
    export QMAKE=${qt.env}/bin/qmake
    export RUSTFLAGS="${qt.rustflags} ''${RUSTFLAGS:-}"
    echo "Ion dev shell: cargo run -p ion-app  (Qt ${qt6.qtbase.version})"
  '';
}
