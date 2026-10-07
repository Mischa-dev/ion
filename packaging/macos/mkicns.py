"""Pack PNG renders of the app icon into a macOS .icns file.

Usage: mkicns.py OUT.icns DIR, where DIR holds icon_<size>.png for every size
below. Modern macOS reads PNG data directly from these icns entries, so no
Apple tooling (iconutil) is needed and the build works in the Nix sandbox.
"""

import struct
import sys
from pathlib import Path

# icns entry type -> pixel size of the PNG stored in it.
ENTRIES = [
    ("icp4", 16),
    ("icp5", 32),
    ("ic11", 32),  # 16pt @2x
    ("icp6", 64),
    ("ic12", 64),  # 32pt @2x
    ("ic07", 128),
    ("ic08", 256),
    ("ic13", 256),  # 128pt @2x
    ("ic09", 512),
    ("ic14", 512),  # 256pt @2x
    ("ic10", 1024),  # 512pt @2x
]
SIZES = sorted({size for _, size in ENTRIES})
PNG_MAGIC = b"\x89PNG\r\n\x1a\n"


def build(png_dir: Path) -> bytes:
    body = b""
    for kind, size in ENTRIES:
        data = (png_dir / f"icon_{size}.png").read_bytes()
        if not data.startswith(PNG_MAGIC):
            raise SystemExit(f"icon_{size}.png is not a PNG")
        width, height = struct.unpack(">II", data[16:24])
        if (width, height) != (size, size):
            raise SystemExit(f"icon_{size}.png is {width}x{height}, want {size}x{size}")
        body += kind.encode() + struct.pack(">I", 8 + len(data)) + data
    return b"icns" + struct.pack(">I", 8 + len(body)) + body


if __name__ == "__main__":
    if len(sys.argv) == 2 and sys.argv[1] == "--sizes":
        print(" ".join(map(str, SIZES)))
    elif len(sys.argv) == 3:
        Path(sys.argv[1]).write_bytes(build(Path(sys.argv[2])))
    else:
        raise SystemExit(__doc__)
