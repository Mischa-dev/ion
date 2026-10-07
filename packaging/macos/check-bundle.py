"""Sanity-check an Ion.app bundle: Info.plist, executable and icon.

Usage: check-bundle.py PATH/TO/Ion.app
"""

import os
import plistlib
import struct
import sys
from pathlib import Path

app = Path(sys.argv[1])
contents = app / "Contents"
info = plistlib.loads((contents / "Info.plist").read_bytes())

for key in ("CFBundleIdentifier", "CFBundleExecutable", "CFBundleIconFile",
            "CFBundleShortVersionString", "NSCameraUsageDescription",
            "NSMicrophoneUsageDescription"):
    assert info.get(key), f"Info.plist lacks {key}"
assert info["CFBundleIdentifier"] == "dev.ion.Ion", info["CFBundleIdentifier"]
assert "@" not in info["CFBundleShortVersionString"], "version was not substituted"

executable = contents / "MacOS" / info["CFBundleExecutable"]
assert executable.is_file() and os.access(executable, os.X_OK), f"{executable} is not executable"

icon = (contents / "Resources" / info["CFBundleIconFile"]).read_bytes()
magic, length = struct.unpack(">4sI", icon[:8])
assert magic == b"icns" and length == len(icon), "Ion.icns is malformed"

print(f"{app}: ok ({info['CFBundleShortVersionString']})")
