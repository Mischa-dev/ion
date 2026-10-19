use crate::{Context, Control, Group, Section, Setting};

pub fn section(_: &Context) -> Section {
    Section::new(
        "new-tab",
        "New tab",
        "▦",
        "The page a new tab opens on.",
        vec![Group::new(
            "Shortcuts",
            vec![
                Setting::new(
                    "newTab.mostVisited",
                    "Show the sites you visit most",
                    Control::Toggle,
                )
                .help("After any shortcuts pinned in your config.")
                .keywords("tiles top sites speed dial"),
                Setting::new(
                    "newTab.tiles",
                    "Shortcuts shown",
                    Control::Number {
                        min: 0.0,
                        max: 24.0,
                        step: 1.0,
                        unit: String::new(),
                        slider: true,
                    },
                )
                .keywords("tiles count speed dial"),
            ],
        )],
    )
}
