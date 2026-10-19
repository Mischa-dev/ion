use crate::{Context, Control, Group, Section, Setting, choices};

pub fn section(_: &Context) -> Section {
    Section::new(
        "gestures",
        "Gestures",
        "layers",
        "Touchpad swipes and pinches, and mouse strokes.",
        vec![
            Group::new(
                "Touchpad",
                vec![
                    Setting::new("gestures.swipe", "Swipe to go back and forward", Control::Toggle)
                        .help("Swipe two fingers sideways past the edge of the page. An arrow shows when letting go will navigate.")
                        .keywords("touchpad trackpad history navigate"),
                    Setting::new(
                        "gestures.pinch",
                        "Pinch",
                        Control::Choice {
                            options: choices(&[
                                ("zoom", "Zoom the page"),
                                ("magnify", "Magnify"),
                                ("off", "Nothing"),
                            ]),
                        },
                    )
                    .help("Zoom is remembered per site, like Ctrl and scroll. Magnify enlarges the spot under your fingers without reflowing the page.")
                    .keywords("touchpad trackpad zoom"),
                ],
            ),
            Group::new(
                "Mouse",
                vec![
                    Setting::new("gestures.mouse", "Mouse gestures", Control::Toggle)
                        .help("Hold the right button and draw: left goes back, right goes forward, down opens a tab, down then right closes it, down then left reopens it, up then down reloads. A plain right-click still opens the menu. Change strokes under [gestures] mouseStrokes.")
                        .keywords("mouse strokes right button draw"),
                ],
            ),
        ],
    )
}
