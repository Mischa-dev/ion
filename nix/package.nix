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
  icons = callPackage ./icons.nix { };
  cargoToml = lib.importTOML ../Cargo.toml;
  inherit (cargoToml.workspace.package) version;
in
rustPlatform.buildRustPackage {
  pname = "ion";
  inherit version;

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

  postInstall =
    if stdenv.hostPlatform.isDarwin then
      ''
        # The real binary lives in the bundle so macOS finds Info.plist (the
        # camera/microphone prompts, URL handling, Dock name and icon).
        contents=$out/Applications/Ion.app/Contents
        install -d $contents/MacOS
        mv $out/bin/ion $contents/MacOS/ion
        substitute ${../packaging/macos/Info.plist} $contents/Info.plist --subst-var-by version ${version}
        install -Dm644 ${icons}/Ion.icns $contents/Resources/Ion.icns
      ''
    else
      ''
        install -Dm644 ${../packaging/linux/dev.ion.Ion.desktop} $out/share/applications/dev.ion.Ion.desktop
        install -Dm644 ${../packaging/linux/dev.ion.Ion.metainfo.xml} $out/share/metainfo/dev.ion.Ion.metainfo.xml
        cp -r ${icons}/share/icons $out/share/
      '';

  # After wrapQtAppsHook has wrapped the bundle's binary, so `ion` on PATH
  # (and `nix run`) starts the same wrapped app.
  postFixup = lib.optionalString stdenv.hostPlatform.isDarwin ''
    ln -s ../Applications/Ion.app/Contents/MacOS/ion $out/bin/ion
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
