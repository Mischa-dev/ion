# Passwords and privacy

## Saved passwords

Ion's password manager lives in `crates/ion-passwords` (logic, no Qt),
`crates/ion-keyring` (secrets in the OS keyring) and
`crates/ion-app/src/bridge/passwords.rs` (the `Passwords` QML singleton).

**Storage.** Logins are kept in `passwords.json` in Ion's data folder.
Usernames and sites are plain text; passwords and notes are encrypted with
AES-256-GCM. The key is one item ("Ion", `passwords.key`) in the system
keyring: the Secret Service on Linux (GNOME Keyring, KWallet, KeePassXC) or
the login Keychain on macOS. Copying `passwords.json` to another machine
gives nothing away without that key. When no keyring answers, the key goes
to `secrets.json` (mode 0600) in the data folder instead, and the manager
says so. `ION_KEYRING=file` forces that.

The vault opens the first time it's needed (a login page, the manager), not
at startup, since opening it may ask to unlock the keyring.

**On pages.** `ion_passwords::PAGE_SCRIPT` runs in an isolated JavaScript
world of each tab's top frame and talks to Ion over a `WebChannel` that only
exists in that world (`FormFill.qml`). Page scripts can't see it, call it, or
read what it fills before it's in a field. Ion checks every message against
the tab's current origin.

- **Autofill**: a sign-in form is filled as the page loads when the site has
  exactly one saved login, the page is https (or localhost) and
  `passwords.autofill` is on. Sign-up and change-password forms never are.
- **Picking**: focusing a login field lists the site's logins (and other
  subdomains of the same site, labelled), driven by the mouse or by
  arrows/Enter/Escape from the field.
- **Generating**: a new-password field offers a 20-character random password
  (no look-alike characters) and fills it and its confirmation.
- **Saving**: sending a form with a password (submit, its button, Enter, or
  leaving the page) offers to save it, or to update a saved one, from the key
  in the toolbar. "Never for this site" remembers the origin.

Logins are matched by origin: `https://a.com` gets logins saved on
`https://a.com` and on `http://a.com`, never the other way round, and other
subdomains of the same registrable domain are offered but not autofilled.

**Manager** (palette: "Passwords", or the key icon): search, reveal, copy
(hidden from clipboard history, cleared after a minute), edit, delete, a
checkup for weak and reused passwords, and CSV import/export. Import reads
exports from Chrome, Edge, Brave, Firefox, Safari, Bitwarden and 1Password.

**Other secrets.** `ion_keyring::global()` stores any named secret
(`get`/`set`/`delete`/`get_or_create`) for the rest of Ion, for example an
agent's API key.
