use crate::{Context, Control, Group, Section, Setting};

pub fn section(_: &Context) -> Section {
    Section::new(
        "keyboard",
        "Keyboard",
        "key",
        "Ways to browse without the mouse.",
        vec![Group::new(
            "",
            vec![
                Setting::new("keyboard.vim", "Vim keys in pages", Control::Toggle)
                    .help("j and k scroll, gg and G jump, H and L go back and forward, f shows link hints (F opens them in a new tab).")
                    .keywords("vimium hints navigation"),
                Setting::new(
                    "keyboard.palette",
                    "Every command and shortcut",
                    Control::Action {
                        action: "command-palette".into(),
                        button: "Open palette".into(),
                    },
                )
                .keywords("shortcuts commands palette ctrl+k"),
                Setting::new(
                    "keyboard.shortcuts",
                    "Keyboard shortcuts",
                    Control::Action {
                        action: "keyboard-shortcuts".into(),
                        button: "Change shortcuts".into(),
                    },
                )
                .help("Pick new keys for any command. They are saved to [shortcuts] in overrides.toml.")
                .keywords("keys keybindings hotkeys remap customize"),
            ],
        )],
    )
}
