use crate::{Context, Control, Group, Section, Setting, choices};

pub fn section(_: &Context) -> Section {
    Section::new(
        "privacy",
        "Privacy",
        "eye-off",
        "What sites can learn about you, and what Ion forgets.",
        vec![
            Group::new(
                "Protections",
                vec![
                    Setting::new(
                        "privacy.httpsOnly",
                        "Secure connections",
                        Control::Choice {
                            options: choices(&[
                                ("upgrade", "HTTPS first"),
                                ("strict", "HTTPS only"),
                                ("off", "Off"),
                            ]),
                        },
                    )
                    .help("HTTPS first loads every site securely when it can and quietly falls back when it can't. HTTPS only asks first.")
                    .keywords("https ssl tls encrypted insecure http upgrade"),
                    Setting::new(
                        "privacy.fingerprinting",
                        "Resist fingerprinting",
                        Control::Toggle,
                    )
                    .help("Sites see slightly different canvas, audio and graphics details each session, so they can't recognize your device. Off on sites whose shield is off.")
                    .keywords("fingerprint canvas webgl tracking"),
                ],
            ),
            Group::new(
                "Tracking",
                vec![
                    Setting::new(
                        "privacy.blockThirdPartyCookies",
                        "Block third-party cookies",
                        Control::Toggle,
                    )
                    .help("Stops sites you aren't visiting from following you across the web. A few sign-in flows may need it off.")
                    .keywords("tracking cookies cross-site"),
                    Setting::new(
                        "privacy.globalPrivacyControl",
                        "Ask sites not to sell or share your data",
                        Control::Toggle,
                    )
                    .help("Sends Global Privacy Control, which some laws require sites to honor.")
                    .keywords("gpc do not track dnt"),
                ],
            ),
            Group::new(
                "Private windows and profiles",
                vec![
                    Setting::new(
                        "privacy.privateWindow",
                        "Private window",
                        Control::Action {
                            action: "new-private-window".into(),
                            button: "Open".into(),
                        },
                    )
                    .help("No history, and cookies and site data are forgotten when it closes. Ctrl+Shift+N.")
                    .keywords("incognito private browsing"),
                    Setting::new(
                        "privacy.profiles",
                        "Profiles",
                        Control::Action {
                            action: "profiles".into(),
                            button: "Manage…".into(),
                        },
                    )
                    .help("Separate windows with their own sign-ins, history and passwords, like Work and Personal.")
                    .keywords("profiles accounts containers work personal"),
                ],
            ),
            Group::new(
                "When Ion quits",
                vec![
                    Setting::new(
                        "privacy.clearOnExit",
                        "Delete",
                        Control::Flags {
                            options: choices(&[
                                ("history", "History"),
                                ("cookies", "Cookies and sign-ins"),
                                ("cache", "Cached files"),
                            ]),
                        },
                    )
                    .help("Open tabs and saved sessions are kept.")
                    .keywords("clear on exit forget wipe"),
                    Setting::new(
                        "privacy.clearNow",
                        "Clear browsing data now",
                        Control::Action {
                            action: "clear-browsing-data".into(),
                            button: "Clear…".into(),
                        },
                    )
                    .keywords("delete history cookies cache forget"),
                ],
            ),
        ],
    )
}
