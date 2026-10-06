# The Qt 6 modules Ion builds against, merged into one prefix so that
# cxx-qt-build (which asks `qmake -query` for paths) can find every module.
{
  lib,
  stdenv,
  qt6,
}:
rec {
  modules = [
    qt6.qtbase
    qt6.qtdeclarative
    qt6.qtwebengine
    qt6.qtwebchannel
    qt6.qtpositioning
    qt6.qtsvg
  ]
  ++ lib.optionals stdenv.hostPlatform.isLinux [ qt6.qtwayland ];

  env = qt6.env "ion-qt-${qt6.qtbase.version}" modules;

  # cxx-qt-build links against the merged prefix but does not set a runtime
  # search path on Linux, so without this the binary finds neither libQt6*.so
  # nor libstdc++ outside a Nix build sandbox.
  rustflags = lib.optionalString stdenv.hostPlatform.isLinux (
    lib.concatMapStringsSep " " (dir: "-C link-arg=-Wl,-rpath,${dir}") [
      "${env}/lib"
      "${lib.getLib stdenv.cc.cc}/lib"
    ]
  );
}
