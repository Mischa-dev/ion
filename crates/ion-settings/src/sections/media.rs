use crate::{Context, Control, Group, Section, Setting, choices};

pub fn section(_: &Context) -> Section {
    Section::new(
        "media",
        "Media",
        "♪",
        "Sound and video on the pages you visit.",
        vec![Group::new(
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
            ],
        )],
    )
}
