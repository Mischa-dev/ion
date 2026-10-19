use crate::{Choice, Context, Control, Group, Section, Setting, choices};

/// "catppuccin-mocha" → "Catppuccin Mocha".
fn display_name(id: &str) -> String {
    id.split(['-', '_'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn section(ctx: &Context) -> Section {
    let mut themes = vec![Choice {
        value: "auto".into(),
        label: "Automatic".into(),
    }];
    themes.extend(ctx.themes.iter().map(|id| Choice {
        value: id.clone(),
        label: display_name(id),
    }));

    Section::new(
        "appearance",
        "Appearance",
        "sun",
        "Colors, layout and motion.",
        vec![
            Group::new(
                "Theme",
                vec![
                    Setting::new(
                        "theme.source",
                        "Colors from",
                        Control::Choice {
                            options: choices(&[
                                ("builtin", "Ion themes"),
                                ("system", "System accent"),
                                ("dms", "DankMaterialShell"),
                                ("manual", "Palette file"),
                            ]),
                        },
                    )
                    .help("DankMaterialShell and palette files are re-read as soon as they change.")
                    .keywords("colors palette dms matugen accent"),
                    Setting::new("theme.name", "Theme", Control::Choice { options: themes })
                        .help("Automatic follows your system's light or dark mode.")
                        .keywords("dark light mode colors"),
                    Setting::new(
                        "theme.pages",
                        "Web pages",
                        Control::Choice {
                            options: choices(&[
                                ("match", "Match Ion"),
                                ("system", "Match system"),
                                ("darken", "Darken all"),
                            ]),
                        },
                    )
                    .help("Whether sites get Ion's light or dark mode. Darken all also darkens sites with no dark style of their own.")
                    .keywords("dark mode force prefers-color-scheme sites"),
                    Setting::new(
                        "theme.pageControls",
                        "Theme scrollbars and form controls on pages",
                        Control::Toggle,
                    )
                    .help("Unless the site styles them itself.")
                    .keywords("accent scrollbar checkbox selection"),
                ],
            ),
            Group::new(
                "Layout",
                vec![
                    Setting::new(
                        "ui.tabs",
                        "Tabs",
                        Control::Choice {
                            options: choices(&[
                                ("horizontal", "Across the top"),
                                ("vertical", "In a sidebar"),
                            ]),
                        },
                    )
                    .keywords("vertical horizontal sidebar tab strip"),
                    Setting::new(
                        "ui.collapseSidebar",
                        "Shrink the tab sidebar to icons",
                        Control::Toggle,
                    )
                    .help("It opens while the pointer is over it.")
                    .keywords("vertical tabs collapse compact"),
                    Setting::new(
                        "ui.density",
                        "Density",
                        Control::Choice {
                            options: choices(&[
                                ("comfortable", "Comfortable"),
                                ("compact", "Compact"),
                            ]),
                        },
                    )
                    .keywords("size spacing compact"),
                    Setting::new(
                        "ui.cornerRadius",
                        "Corner roundness",
                        Control::Number {
                            min: 0.0,
                            max: 16.0,
                            step: 1.0,
                            unit: "px".into(),
                            slider: true,
                        },
                    )
                    .keywords("radius rounded square corners"),
                ],
            ),
            Group::new(
                "Motion",
                vec![
                    Setting::new("ui.animations.enable", "Animations", Control::Toggle)
                        .keywords("motion reduce transitions"),
                    Setting::new(
                        "ui.animations.speed",
                        "Animation speed",
                        Control::Number {
                            min: 0.25,
                            max: 3.0,
                            step: 0.25,
                            unit: "×".into(),
                            slider: true,
                        },
                    )
                    .help("2× is twice as fast.")
                    .keywords("motion fast slow")
                    .requires("ui.animations.enable"),
                ],
            ),
        ],
    )
}
