# Platform integration and packaging

What Ion does per platform, and how to check it on real hardware. The cloud CI
builds both platforms but cannot run a GPU, a Wayland compositor or a Mac
desktop, so the checklists below are manual.

## Where things live

| Piece | Files |
|---|---|
| App icon (source of every render) | `crates/ion-app/icons/dev.ion.Ion.svg`, rendered by `nix/icons.nix` |
| Linux desktop file, AppStream metadata | `packaging/linux/` |
| macOS bundle (`Ion.app`) | `packaging/macos/Info.plist`, `mkicns.py`, assembled in `nix/package.nix` |
| Chromium switches (hardware video) | `crates/ion-platform/src/chromium.rs` |
| One Ion per profile, URL hand-off | `crates/ion-platform/src/instance.rs`, `bridge/platform.rs` |
| Window icon, macOS open-URL events, Wayland activation | `crates/ion-app/cpp/platform.{h,cpp}` |
| macOS-only shortcuts, opening handed-over URLs | `qml/components/PlatformIntegration.qml` |
| Package checks | `nix/packaging-check.nix` (`nix flake check`) |

## One Ion per profile

The first Ion binds `$XDG_RUNTIME_DIR/dev.ion.Ion/Default.sock` (on macOS a
per-user directory under `$TMPDIR`). A later `ion URL…`, such as a link clicked
in another app, sends its URLs there and exits; the running window opens them
as tabs and comes to the front. Two processes cannot share one QtWebEngine
profile, so without this a second launch would fail to load the profile.

On Wayland the launcher's `XDG_ACTIVATION_TOKEN` travels with the request, so
compositors that enforce focus-stealing prevention still let the window come
forward. On macOS, clicked links and opened files arrive as Apple events, which
reach the same path through `QFileOpenEvent`.

## Hardware video and GPU (Linux)

Ion turns on Chromium's VA-API features through `QTWEBENGINE_CHROMIUM_FLAGS`
and merges them with whatever you already set there; your own
`--disable-features=…` wins. `ION_HARDWARE_VIDEO=0` turns Ion's defaults off.
The nixpkgs QtWebEngine build links libva and enables proprietary codecs
(H.264, AAC), which these features need.

Checklist on a real machine:

1. `vainfo` lists decode profiles (on NixOS: `hardware.graphics.enable = true`
   plus the VA-API driver for your GPU, for example `intel-media-driver`).
2. Start Ion from a Wayland session and open `chrome://gpu`.
   - "Command Line" shows `--enable-features=…AcceleratedVideoDecodeLinuxGL…`.
   - Under "Graphics Feature Status", Compositing and Rasterization read
     "Hardware accelerated", and "Video Decode" reads "Hardware accelerated".
3. Play a 1080p+ video, then open `chrome://media-internals`: the player's
   `kVideoDecoderName` should be `VaapiVideoDecoder` (or `VDAVideoDecoder`),
   not `FFmpegVideoDecoder`/`VpxVideoDecoder`.
4. `QT_QPA_PLATFORM` unset on a Wayland session should give a native Wayland
   window (`xlsclients` does not list Ion; the compositor's window list shows
   app id `dev.ion.Ion` with the Ion icon).

If decoding stays in software, try `QTWEBENGINE_CHROMIUM_FLAGS=--ignore-gpu-blocklist`,
and note the GPU, driver and what `chrome://gpu` says in an issue.

## macOS

`nix build` produces `result/Applications/Ion.app` (plus `result/bin/ion`,
which starts the same bundle). The bundle carries the camera, microphone and
location usage strings that macOS requires before a site can ask for them, and
registers Ion for `http`/`https` links and HTML files so it can be picked as
the default browser.

Qt maps `Ctrl` in shortcuts to Cmd and `Meta` to the Control key. Shortcut
check:

| Action | macOS keys | Defined in |
|---|---|---|
| New tab | Cmd+T | `Main.qml` (`StandardKey.AddTab`) |
| Close tab | Cmd+W | `Main.qml` (`StandardKey.Close`) |
| Focus address bar | Cmd+L | `Main.qml` |
| Reload | Cmd+R | `Main.qml` (`StandardKey.Refresh`) |
| Back / forward | Cmd+[ / Cmd+], Cmd+Left / Cmd+Right | `Main.qml` (`StandardKey.Back/Forward`) |
| Reopen closed tab | Cmd+Shift+T | `Main.qml` |
| Go to tab 1–8 / last | Cmd+1…Cmd+8 / Cmd+9 | `Main.qml` |
| Quit | Cmd+Q | `Main.qml` and the native app menu |
| Next / previous tab | Control+Tab / Control+Shift+Tab, Cmd+Shift+] / [, Cmd+Option+Right / Left | `PlatformIntegration.qml` |

The shared `Ctrl+Tab` becomes Cmd+Tab on macOS, which the system keeps for
app switching; that is why the macOS tab-cycling keys are added separately.
Still to check on a Mac: the Cmd+Shift+] / [ pair (Qt matches them as
`Ctrl+}` / `Ctrl+{`).
