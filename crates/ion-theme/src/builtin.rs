//! Themes shipped inside the binary. Each is an ordinary palette file under
//! `themes/`, so they double as examples of the format.

use crate::palette::Palette;

/// Used when the system is in dark mode (or its mode is unknown).
pub const DEFAULT_DARK: &str = "ion-dark";
/// Used when the system is in light mode.
pub const DEFAULT_LIGHT: &str = "ion-light";

const THEMES: &[(&str, &str)] = &[
    ("ion-dark", include_str!("../themes/ion-dark.toml")),
    ("ion-light", include_str!("../themes/ion-light.toml")),
    (
        "catppuccin-mocha",
        include_str!("../themes/catppuccin-mocha.toml"),
    ),
    (
        "catppuccin-latte",
        include_str!("../themes/catppuccin-latte.toml"),
    ),
    ("nord", include_str!("../themes/nord.toml")),
];

/// Ids of the built-in themes, in display order.
pub fn names() -> impl Iterator<Item = &'static str> {
    THEMES.iter().map(|(id, _)| *id)
}

/// A built-in theme by id, e.g. `"nord"`. `"dark"` and `"light"` are short
/// for Ion's own themes.
pub fn get(id: &str) -> Option<Palette> {
    let id = match id {
        "dark" => DEFAULT_DARK,
        "light" => DEFAULT_LIGHT,
        id => id,
    };
    THEMES.iter().find(|(name, _)| *name == id).map(|(_, src)| {
        Palette::from_toml(src, id).unwrap_or_else(|e| panic!("built-in theme {id}: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::Scheme;

    #[test]
    fn every_builtin_parses_and_is_readable() {
        for id in names() {
            let p = get(id).unwrap();
            assert!(
                p.text.contrast(p.background) >= 4.5,
                "{id}: text on background"
            );
            assert!(p.text.contrast(p.surface) >= 4.5, "{id}: text on surface");
            assert!(
                p.text_muted.contrast(p.surface) >= 3.0,
                "{id}: muted text on surface"
            );
            assert!(
                p.on_accent.contrast(p.accent) >= 4.5,
                "{id}: text on accent"
            );
        }
    }

    #[test]
    fn defaults_match_their_scheme() {
        assert_eq!(get(DEFAULT_DARK).unwrap().scheme, Scheme::Dark);
        assert_eq!(get(DEFAULT_LIGHT).unwrap().scheme, Scheme::Light);
    }

    #[test]
    fn short_names_pick_ion_themes() {
        assert_eq!(get("dark"), get(DEFAULT_DARK));
        assert_eq!(get("light"), get(DEFAULT_LIGHT));
    }

    #[test]
    fn unknown_theme_is_none() {
        assert!(get("no-such-theme").is_none());
    }
}
