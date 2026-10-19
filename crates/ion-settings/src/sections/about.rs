use crate::{Context, Control, Group, Section, Setting};

pub fn section(_: &Context) -> Section {
    let action = |action: &str, button: &str| Control::Action {
        action: action.into(),
        button: button.into(),
    };
    Section::new(
        "config",
        "Config file",
        "⚙",
        "Everything here is also plain TOML. Changes made on this page go to overrides.toml and win over config.toml, which Ion never writes, so a home-manager config keeps working.",
        vec![Group::new(
            "",
            vec![
                Setting::new(
                    "config.openFolder",
                    "Config folder",
                    action("open-config-folder", "Open"),
                )
                .help("config.toml, overrides.toml, themes and user scripts.")
                .keywords("toml directory files"),
                Setting::new(
                    "config.copyNix",
                    "Copy your changes as Nix",
                    action("copy-nix", "Copy"),
                )
                .help("A programs.ion snippet for your home-manager config. Once it's in, reset the changes here.")
                .keywords("home-manager nixos promote export"),
                Setting::new(
                    "config.resetAll",
                    "Reset every change made here",
                    action("reset-all", "Reset…"),
                )
                .help("Deletes overrides.toml, so config.toml and Ion's defaults apply again.")
                .keywords("defaults restore clear"),
            ],
        )],
    )
}
