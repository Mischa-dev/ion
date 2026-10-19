use crate::{Context, Control, Group, Section, Setting};

pub fn section(_: &Context) -> Section {
    Section::new(
        "passwords",
        "Passwords and autofill",
        "⚿",
        "Saved sign-ins and addresses, encrypted with a key in your system keyring.",
        vec![
            Group::new(
                "Passwords",
                vec![
                    Setting::new(
                        "passwords.enable",
                        "Offer to save passwords and addresses",
                        Control::Toggle,
                    )
                    .help("Ion asks after you sign in or send an address form. Turn off if you use another password manager.")
                    .keywords("save logins credentials vault manager"),
                    Setting::new(
                        "passwords.autofill",
                        "Sign in automatically",
                        Control::Toggle,
                    )
                    .help("Fills a site's only saved login as its page loads, on secure pages. Otherwise pick one under the field.")
                    .keywords("autofill auto fill login")
                    .requires("passwords.enable"),
                    Setting::new(
                        "passwords.manage",
                        "Saved passwords",
                        Control::Action {
                            action: "passwords".into(),
                            button: "Manage…".into(),
                        },
                    )
                    .help("Search, copy, edit, check for weak or reused ones, and import from Chrome, Firefox, Safari, Bitwarden or 1Password.")
                    .keywords("logins import export csv checkup weak reused"),
                    Setting::new(
                        "passwords.generate",
                        "Copy a strong new password",
                        Control::Action {
                            action: "generate-password".into(),
                            button: "Copy".into(),
                        },
                    )
                    .keywords("generate random generator"),
                ],
            ),
            Group::new(
                "Addresses",
                vec![
                    Setting::new(
                        "passwords.addresses",
                        "Saved addresses",
                        Control::Action {
                            action: "addresses".into(),
                            button: "Manage…".into(),
                        },
                    )
                    .help("Listed under address fields to fill whole forms: shipping, billing and contact details.")
                    .keywords("autofill address shipping billing phone email forms"),
                ],
            ),
        ],
    )
}
