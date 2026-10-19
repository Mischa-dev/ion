use crate::{Context, Control, Group, Section, Setting, choices};

pub fn section(_: &Context) -> Section {
    Section::new(
        "privacy",
        "Privacy",
        "eye-off",
        "What sites can learn about you, and what Ion forgets.",
        vec![
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
