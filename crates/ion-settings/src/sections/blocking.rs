use crate::{Context, Control, Group, Section, Setting};

pub fn section(_: &Context) -> Section {
    Section::new(
        "blocking",
        "Ads and trackers",
        "⊘",
        "Ion blocks ads and trackers on every site, before they load.",
        vec![Group::new(
            "",
            vec![
                Setting::new("adblock.enable", "Block ads and trackers", Control::Toggle)
                    .help("Turn it off for one site from the shield in the address bar.")
                    .keywords("adblock ublock content blocker"),
                Setting::new(
                    "adblock.lists",
                    "Filter lists",
                    Control::List {
                        placeholder: "Add a list name or https:// URL".into(),
                    },
                )
                .help("easylist, easyprivacy, ublock-filters and ublock-privacy are built in; any other list works by URL.")
                .keywords("easylist easyprivacy ublock filters subscriptions")
                .requires("adblock.enable"),
                Setting::new(
                    "adblock.update",
                    "Update filter lists now",
                    Control::Action {
                        action: "update-filter-lists".into(),
                        button: "Update".into(),
                    },
                )
                .keywords("refresh download")
                .requires("adblock.enable"),
            ],
        )],
    )
}
