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

## Crashes

A tab whose page process dies shows Ion's page ("youtube.com stopped working")
with Try again; other tabs keep going. Ion marks itself running in
`<data dir>/sessions/running` and clears the mark on a clean quit. If the mark
is still there at the next start, the last run crashed: with
`general.restoreSession` on (the default) the tabs come back as usual; with it
off Ion offers "Restore tabs" from a copy of that run's session.

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
6. Crash: open `chrome://crash` in a tab; the tab shows the "stopped working"
   page and Try again reloads. `pkill -9 ion`, start Ion again: the tabs are
   back (or offered, with `restoreSession = false`).
