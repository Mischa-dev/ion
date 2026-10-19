use crate::{Context, Control, Group, Section, Setting, choices};

pub fn section(_: &Context) -> Section {
    Section::new(
        "media",
        "Media",
        "♪",
        "Sound, video and reading on the pages you visit.",
        vec![
            Group::new(
            "",
            vec![
                Setting::new(
                    "media.autoplay",
                    "Autoplay",
                    Control::Choice {
                        options: choices(&[
                            ("muted", "Silent video only"),
                            ("allow", "Allow"),
                            ("block", "Block"),
                        ]),
                    },
                )
                .help("What pages may play before you click or type. Silent video only lets muted previews play while sound waits for you; Block stops even those. Sites can differ under [sites] in config.")
                .keywords("autoplay video audio sound play mute"),
                Setting::new(
                    "media.autoPictureInPicture",
                    "Picture-in-picture when you switch tabs",
                    Control::Toggle,
                )
                .help("A playing video pops out into its own small window when you leave its tab, and goes back when you return.")
                .keywords("pip picture in picture floating video window"),
            ],
        ),
        Group::new(
            "Reader mode",
            vec![
                Setting::new(
                    "reader.fontSize",
                    "Text size",
                    Control::Number {
                        min: 12.0,
                        max: 32.0,
                        step: 1.0,
                        unit: "px".into(),
                        slider: true,
                    },
                )
                .keywords("reader font size larger smaller"),
                Setting::new(
                    "reader.font",
                    "Typeface",
                    Control::Choice {
                        options: choices(&[("serif", "Serif"), ("sans", "Sans")]),
                    },
                )
                .keywords("reader font serif sans"),
                Setting::new(
                    "reader.width",
                    "Column width",
                    Control::Choice {
                        options: choices(&[
                            ("narrow", "Narrow"),
                            ("medium", "Medium"),
                            ("wide", "Wide"),
                        ]),
                    },
                )
                .keywords("reader width line length"),
                Setting::new(
                    "reader.theme",
                    "Colors",
                    Control::Choice {
                        options: choices(&[
                            ("match", "Ion's theme"),
                            ("light", "Light"),
                            ("sepia", "Sepia"),
                            ("dark", "Dark"),
                        ]),
                    },
                )
                .help("The reader's toolbar changes these too.")
                .keywords("reader sepia paper dark light"),
            ],
        ),
        ],
    )
}
