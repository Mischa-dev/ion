# The app icon rendered from crates/ion-app/icons/dev.ion.Ion.svg: a hicolor
# theme tree for Linux and Ion.icns for the macOS bundle.
{
  runCommand,
  librsvg,
  python3,
}:
runCommand "ion-icons"
  {
    nativeBuildInputs = [
      librsvg
      python3
    ];
  }
  ''
    svg=${../crates/ion-app/icons/dev.ion.Ion.svg}
    mkicns=${../packaging/macos/mkicns.py}

    hicolor=$out/share/icons/hicolor
    install -Dm644 $svg $hicolor/scalable/apps/dev.ion.Ion.svg
    for size in 16 24 32 48 64 128 256 512; do
      install -d $hicolor/''${size}x''${size}/apps
      rsvg-convert -w $size -h $size $svg -o $hicolor/''${size}x''${size}/apps/dev.ion.Ion.png
    done

    mkdir pngs
    for size in $(python3 $mkicns --sizes); do
      rsvg-convert -w $size -h $size $svg -o pngs/icon_$size.png
    done
    python3 $mkicns $out/Ion.icns pngs
  ''
