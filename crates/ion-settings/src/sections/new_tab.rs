use crate::{Context, Control, Group, Section, Setting};

pub fn section(_: &Context) -> Section {
    Section::new(
        "new-tab",
        "New tab",
        "grid",
        "The page a new tab opens on.",
        vec![Group::new(
            "Shortcuts",
            vec![
                Setting::new(
                    "newTab.shortcuts",
                    "Pinned shortcuts",
                    Control::Custom {
                        component: "NewTabTiles".into(),
                    },
                )
                .help("Always shown first, in this order.")
                .keywords("tiles pin speed dial favorites"),
                Setting::new(
                    "newTab.mostVisited",
                    "Show the sites you visit most",
                    Control::Toggle,
                )
                .help("After your pinned shortcuts.")
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
