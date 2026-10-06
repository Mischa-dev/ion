{
  lib,
  stdenv,
  callPackage,
  rustPlatform,
  qt6,
  src,
}:
let
  qt = callPackage ./qt.nix { };
  cargoToml = lib.importTOML ../Cargo.toml;
in
rustPlatform.buildRustPackage {
  pname = "ion";
  inherit (cargoToml.workspace.package) version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../crates
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [
    qt6.wrapQtAppsHook
    qt.env
  ];
  buildInputs = qt.modules;

  # qtbase's setup hook points QMAKE at qtbase alone; cxx-qt-build needs the
  # merged prefix so it can find QtQml, QtWebEngine and friends.
  preConfigure = ''
    export QMAKE=${qt.env}/bin/qmake
    export RUSTFLAGS="${qt.rustflags} ''${RUSTFLAGS:-}"
    export CXXFLAGS="${qt.cxxflags} ''${CXXFLAGS:-}"
  '';

  cargoBuildFlags = [
    "--package"
    "ion-app"
  ];
  # Run every crate's unit tests, not just the app's.
  cargoTestFlags = [ "--workspace" ];

  postInstall = lib.optionalString stdenv.hostPlatform.isLinux ''
    install -Dm644 ${../packaging/linux/dev.ion.Ion.desktop} $out/share/applications/dev.ion.Ion.desktop
  '';

  meta = {
    description = "Fast, themeable desktop browser built on QtWebEngine";
    homepage = "https://github.com/Mischa-dev/ion";
    mainProgram = "ion";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
    ];
  };
}
