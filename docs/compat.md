# Site compatibility

What Ion does so real sites work, and how to check it on a real machine. The
cloud CI can't sign in to Google or play DRM video, so the checklists are
manual.

## Where things live

| Piece | Files |
|---|---|
| User agent, built-in site quirks | `crates/ion-sites/src/agent.rs`, `bridge/sites.rs`, `SiteScripts.qml` |
| Links that open other apps | `crates/ion-sites/src/protocols.rs`, `bridge/protocols.rs`, `ExternalAppPrompt.qml` |
| Security keys and passkeys | `crates/ion-basics/src/webauthn.rs`, `bridge/webauthn.rs`, `WebAuthDialog.qml` |
| Crashed tabs | `ion_core::load_error::crashed`, `LoadErrorPage.qml` |
| Restore after a crash | `SessionStore::begin_run`/`end_run` in `ion-session`, `CrashRestoreNotice.qml` |
| Widevine (Netflix, Spotify) | `packages.ion-widevine` in `flake.nix`, `nix/package.nix`, `ion-platform::chromium` |
| Screen sharing on Wayland | `ion-platform::chromium` (PipeWire capturer), `ScreenSharePicker.qml` |
| Desktop notifications | `cpp/notifications.{h,cpp}`, `bridge/notifications.rs`, `NotificationToasts.qml` |
| Portal file pickers | `ion-platform::desktop`, `bridge/platform.rs` |

## User agent

QtWebEngine's user agent carries a `QtWebEngine/6.x` token. Sites read it as
an embedded app: Google sign-in refuses it ("This browser or app may not be
secure") and some sites say the browser isn't supported. Ion drops the token
from the profile's user agent, so every page sees Chromium of the engine's
version, which matches the client hints the engine already sends
(`Sec-CH-UA: "Chromium";v="140"`, no QtWebEngine brand).

Google sign-in still rejects Chromium builds it can't identify as Chrome, so
`accounts.google.com` gets a Firefox user agent (and no client hints) built in,
as qutebrowser does. Config wins: `[sites."accounts.google.com"] userAgent =
"default"` turns the quirk off.

## Links to other apps

`mailto:`, `zoommtg:`, `slack:` and any other scheme the engine doesn't load
itself ask first ("zoom.us wants to open Zoom."), with "Always let zoom.us open
zoommtg: links". Approvals live in `<data dir>/protocols.json`. Sites that ask
to handle a scheme (`navigator.registerProtocolHandler`, such as Gmail for
`mailto:`) get a prompt too; once allowed, those links open in that site.

## Popups and files

Pages a script opened (a "Sign in with Google" or PayPal popup) open as tabs
next to their opener; when the page closes itself with `window.close()`, the
tab goes and the opener comes back. Sites that want to keep using a file or
folder picked for them (the File System Access API: vscode.dev "Open
Folder", Photopea) ask first: "Let vscode.dev see and change the files in
~/code?".

## Crashes

A tab whose page process dies shows Ion's page ("youtube.com stopped working")
with Try again; other tabs keep going. Ion marks itself running in
`<data dir>/sessions/running` and clears the mark on a clean quit. If the mark
is still there at the next start, the last run crashed: with
`general.restoreSession` on (the default) the tabs come back as usual; with it
off Ion offers "Restore tabs" from a copy of that run's session.

## DRM video (Netflix, Spotify, Disney+)

These need Google's Widevine module, which is unfree, so the default `ion`
package doesn't ship it. `packages.ion-widevine` (Linux) does: its wrapper sets
`ION_WIDEVINE_PATH`, and Ion passes it to QtWebEngine as `--widevine-path`.
Install it with `nix profile install github:Mischa-dev/ion#ion-widevine`
(replacing `ion`), or set `programs.ion.widevine = true;` in home-manager.
Like every Linux Chromium, Netflix plays at up to 720p in the browser.

## Calls and screen sharing

On Wayland the screen is only reachable through xdg-desktop-portal and
PipeWire. Ion turns on Chromium's `WebRTCPipeWireCapturer`; sharing opens the
desktop's own picker (from xdg-desktop-portal-gnome, -kde, -wlr or -hyprland)
after Ion's prompt. Camera and microphone ask through Ion's permission prompt.

## Notifications and file pickers

Site notifications go to the desktop's notification server
(org.freedesktop.Notifications, so DMS, mako, dunst, GNOME and KDE all show
them) with the site's icon, and clicking one brings Ion forward and tells the
page. Without a server they stay as Ion's toasts.

Ion sets `QT_QPA_PLATFORMTHEME=xdgdesktopportal` on Linux unless a theme is
already set, so "Upload" and "Save as" use the desktop's file picker through
the portal. Without a portal Qt's own dialog shows instead.

## Checklist on a real machine

1. Google sign-in: sign in at `accounts.google.com` and from YouTube's
   Sign in button. No "browser may not be secure" page; 2-step prompts work.
2. Gmail, Google Docs: Docs opens, typing and copy/paste with Ctrl+C/V work;
   the Edit menu's paste asks for clipboard access once.
3. `mailto:` link: Ion asks, Open starts the desktop mail app (or Gmail after
   allowing Gmail's "Open email links" prompt in Gmail's settings).
4. Zoom or Teams meeting link: Ion asks before starting the desktop app.
5. Security key (YubiKey): register one at <https://webauthn.io>, then sign
   in. The PIN dialog and "Touch your security key" show in Ion's style. If
   the key is never found on NixOS, add `services.udev.packages = [
   pkgs.libfido2 ];` so the logged-in user can open it.
6. Netflix and Spotify with `ion-widevine`: `chrome://media-internals`
   shows a `com.widevine.alpha` CDM, Netflix plays, and Spotify's web player
   doesn't ask to enable protected content.
7. Google Meet or Discord call: camera and mic prompts, then share a screen
   and a single window; the desktop's portal picker shows, and the other side
   sees the share.
8. Discord or Gmail notification while Ion is in the background: it shows as
   a desktop notification, and clicking it brings Ion forward on that chat.
9. An "Upload" button opens the desktop's file picker (GTK or KDE look).
   On vscode.dev, Open Folder asks to see and change the files, then works.
   "Sign in with Google" on a third-party site opens a tab that closes
   itself after signing in.
10. Crash: open `chrome://crash` in a tab; the tab shows the "stopped working"
   page and Try again reloads. `pkill -9 ion`, start Ion again: the tabs are
   back (or offered, with `restoreSession = false`).
