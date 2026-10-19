use crate::{Context, Control, Group, Section, Setting};

pub fn section(_: &Context) -> Section {
    Section::new(
        "downloads",
        "Downloads",
        "↓",
        "Where downloaded files go.",
        vec![Group::new(
            "",
            vec![
                Setting::new(
                    "downloads.directory",
                    "Save files to",
                    Control::Folder {
                        placeholder: "Your Downloads folder".into(),
                    },
                )
                .help("Empty uses the system's download folder.")
                .keywords("folder location path save"),
                Setting::new(
                    "downloads.showAll",
                    "Every download",
                    Control::Action {
                        action: "open:ion://downloads".into(),
                        button: "Open".into(),
                    },
                )
                .keywords("list history files"),
            ],
        )],
    )
}
