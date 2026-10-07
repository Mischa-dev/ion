# Checks that the installed desktop integration is well formed: the desktop
# file and AppStream metadata on Linux, the Ion.app bundle on macOS.
{
  lib,
  stdenv,
  runCommand,
  desktop-file-utils,
  appstream,
  python3,
  ion,
}:
runCommand "ion-packaging-check"
  {
    nativeBuildInputs = [
      python3
    ]
    ++ lib.optionals stdenv.hostPlatform.isLinux [
      desktop-file-utils
      appstream
    ];
  }
  (
    if stdenv.hostPlatform.isDarwin then
      ''
        python3 ${../packaging/macos/check-bundle.py} ${ion}/Applications/Ion.app
        test -x ${ion}/bin/ion
        touch $out
      ''
    else
      ''
        desktop-file-validate ${ion}/share/applications/dev.ion.Ion.desktop
        appstreamcli validate --no-net --explain ${ion}/share/metainfo/dev.ion.Ion.metainfo.xml
        for size in 16x16 32x32 48x48 128x128 256x256 512x512 scalable; do
          test -s ${ion}/share/icons/hicolor/$size/apps/dev.ion.Ion.*
        done
        touch $out
      ''
  )
