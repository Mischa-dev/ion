use crate::{Context, Control, Group, Section, Setting};

pub fn section(_: &Context) -> Section {
    Section::new(
        "general",
        "General",
        "home",
        "How Ion starts and looks after your tabs.",
        vec![
            Group::new(
                "On startup",
                vec![
                    Setting::new(
                        "general.restoreSession",
                        "Reopen the tabs from last time",
                        Control::Toggle,
                    )
                    .help("Your tabs and workspaces come back as you left them.")
                    .keywords("restore session startup launch continue"),
                    Setting::new(
                        "general.homePage",
                        "Home page",
                        Control::Text {
                            placeholder: "https://duckduckgo.com/".into(),
                        },
                    )
                    .help("Opened when there is nothing to restore.")
                    .keywords("start page homepage url"),
                ],
            ),
            Group::new(
                "Tabs",
                vec![
                    Setting::new(
                        "general.suspendTabsAfter",
                        "Sleep unused tabs after",
                        Control::Number {
                            min: 0.0,
                            max: 240.0,
                            step: 5.0,
                            unit: "min".into(),
                            slider: true,
                        },
                    )
                    .help("Frees memory from tabs you haven't looked at; they reload when you return. 0 keeps every tab awake.")
                    .keywords("suspend discard memory performance sleep"),
                ],
            ),
            Group::new(
                "Extensions",
                vec![
                    Setting::new(
                        "extensions.manage",
                        "Chrome extensions",
                        Control::Action {
                            action: "extensions".into(),
                            button: "Manage extensions".into(),
                        },
                    )
                    .help("Unpacked Manifest V3 extensions, from Ion's extensions folder.")
                    .keywords("addons add-ons plugins chrome mv3 ublock"),
                    Setting::new(
                        "extensions",
                        "More extension folders",
                        Control::List {
                            placeholder: "/path/to/unpacked-extension".into(),
                        },
                    )
                    .help("Loaded at startup, on top of Ion's own extensions folder.")
                    .keywords("addons unpacked folder path"),
                ],
            ),
        ],
    )
}
