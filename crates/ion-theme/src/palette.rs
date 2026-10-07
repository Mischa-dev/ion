//! The palette: every color token Ion's UI reads, and the palette file format.
//!
//! A palette file is a small TOML document. Only `background`, `text` and
//! `accent` are required; every other token is derived from them, so DMS
//! templates and community themes can stay tiny:
//!
//! ```toml
//! name = "Kanagawa"          # optional, shown in theme pickers
//! scheme = "dark"            # optional, inferred from `background`
//!
//! [colors]
//! background = "#16161d"
//! text = "#dcd7ba"
//! accent = "#7e9cd8"
//! # surface, surface_raised, surface_hover, border, text_muted,
//! # on_accent, danger, warning, success: optional overrides
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::color::Color;

/// Light or dark. Decides derived colors, `Theme.dark`, and (with
/// `theme.pages`, see [`crate::PageTheming`]) what pages see as
/// `prefers-color-scheme`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    Light,
    Dark,
}

/// A fully resolved set of color tokens. Field names match the `Theme` QML
/// singleton's properties (in camelCase there).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette {
    pub name: String,
    pub scheme: Scheme,
    /// Window and tab strip background.
    pub background: Color,
    /// Toolbars and the active tab.
    pub surface: Color,
    /// Inputs, pressed buttons, hovered tabs.
    pub surface_raised: Color,
    /// Hovered buttons.
    pub surface_hover: Color,
    pub border: Color,
    pub text: Color,
    /// Secondary text, placeholders, inactive tabs.
    pub text_muted: Color,
    /// Focus rings, selection, progress, primary buttons.
    pub accent: Color,
    /// Text and icons drawn on top of `accent`.
    pub on_accent: Color,
    pub danger: Color,
    pub warning: Color,
    pub success: Color,
}

/// Why a palette file could not be used.
#[derive(Debug)]
pub struct PaletteError(String);

impl fmt::Display for PaletteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PaletteError {}

/// The on-disk shape of a palette file. Missing tokens are derived in
/// [`PaletteFile::resolve`].
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaletteFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<Scheme>,
    pub colors: PaletteColors,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaletteColors {
    pub background: Option<Color>,
    pub text: Option<Color>,
    pub accent: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_raised: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_hover: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_muted: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_accent: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub danger: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<Color>,
}

impl PaletteFile {
    /// Fill in every token that the file leaves out.
    pub fn resolve(self, fallback_name: &str) -> Result<Palette, PaletteError> {
        let c = self.colors;
        let missing = |token: &str| PaletteError(format!("palette is missing colors.{token}"));
        let background = c.background.ok_or_else(|| missing("background"))?;
        let text = c.text.ok_or_else(|| missing("text"))?;
        let accent = c.accent.ok_or_else(|| missing("accent"))?;
        let scheme = self.scheme.unwrap_or(if background.is_dark() {
            Scheme::Dark
        } else {
            Scheme::Light
        });

        // Surfaces step from the background towards the text color, so they
        // work for any hue in both light and dark schemes.
        let step = |t: f32| background.mix(text, t);
        let surface_raised = c.surface_raised.unwrap_or_else(|| step(0.10));
        let (danger, warning, success) = match scheme {
            Scheme::Dark => (
                Color::rgb(0xe4, 0x68, 0x76),
                Color::rgb(0xe6, 0xc3, 0x84),
                Color::rgb(0x98, 0xbb, 0x6c),
            ),
            Scheme::Light => (
                Color::rgb(0xc4, 0x37, 0x4a),
                Color::rgb(0xa9, 0x6b, 0x00),
                Color::rgb(0x3f, 0x7a, 0x28),
            ),
        };

        Ok(Palette {
            name: self.name.unwrap_or_else(|| fallback_name.to_owned()),
            scheme,
            background,
            surface: c.surface.unwrap_or_else(|| step(0.05)),
            surface_raised,
            surface_hover: c.surface_hover.unwrap_or_else(|| step(0.16)),
            border: c.border.unwrap_or(surface_raised),
            text,
            text_muted: c.text_muted.unwrap_or_else(|| text.mix(background, 0.4)),
            accent,
            on_accent: c.on_accent.unwrap_or_else(|| accent.readable_text()),
            danger: c.danger.unwrap_or(danger),
            warning: c.warning.unwrap_or(warning),
            success: c.success.unwrap_or(success),
        })
    }
}

impl Palette {
    /// Parse a palette file. `fallback_name` is used when it has no `name`.
    pub fn from_toml(source: &str, fallback_name: &str) -> Result<Palette, PaletteError> {
        let file: PaletteFile =
            toml::from_str(source).map_err(|e| PaletteError(e.message().to_owned()))?;
        file.resolve(fallback_name)
    }

    /// The same palette with a different accent, e.g. the system accent color.
    pub fn with_accent(mut self, accent: Color) -> Palette {
        let accent = Color { a: 0xff, ..accent };
        self.accent = accent;
        self.on_accent = accent.readable_text();
        self
    }

    /// Write the palette out in palette-file form, with every token spelled out.
    pub fn to_toml(&self) -> String {
        let file = PaletteFile {
            name: Some(self.name.clone()),
            scheme: Some(self.scheme),
            colors: PaletteColors {
                background: Some(self.background),
                text: Some(self.text),
                accent: Some(self.accent),
                surface: Some(self.surface),
                surface_raised: Some(self.surface_raised),
                surface_hover: Some(self.surface_hover),
                border: Some(self.border),
                text_muted: Some(self.text_muted),
                on_accent: Some(self.on_accent),
                danger: Some(self.danger),
                warning: Some(self.warning),
                success: Some(self.success),
            },
        };
        toml::to_string(&file).expect("a palette always serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{BLACK, WHITE};

    const MINIMAL: &str = r##"
        [colors]
        background = "#16161d"
        text = "#dcd7ba"
        accent = "#7e9cd8"
    "##;

    #[test]
    fn derives_missing_tokens() {
        let p = Palette::from_toml(MINIMAL, "custom").unwrap();
        assert_eq!(p.name, "custom");
        assert_eq!(p.scheme, Scheme::Dark);
        assert_eq!(p.border, p.surface_raised);
        assert_eq!(p.on_accent, BLACK);
        // Surfaces get progressively closer to the text color.
        let d = |c: Color| c.contrast(p.background);
        assert!(d(p.surface) < d(p.surface_raised));
        assert!(d(p.surface_raised) < d(p.surface_hover));
        assert!(p.text_muted.contrast(p.background) < p.text.contrast(p.background));
    }

    #[test]
    fn infers_light_scheme() {
        let p = Palette::from_toml(
            "[colors]\nbackground = \"#fafafa\"\ntext = \"#222\"\naccent = \"#0060df\"",
            "x",
        )
        .unwrap();
        assert_eq!(p.scheme, Scheme::Light);
        assert_eq!(p.on_accent, WHITE);
    }

    #[test]
    fn explicit_tokens_win() {
        let src = format!("name = \"Mine\"\nscheme = \"light\"\n{MINIMAL}\nsurface = \"#ff0000\"");
        let p = Palette::from_toml(&src, "x").unwrap();
        assert_eq!(p.name, "Mine");
        assert_eq!(p.scheme, Scheme::Light);
        assert_eq!(p.surface, Color::rgb(0xff, 0, 0));
    }

    #[test]
    fn reports_missing_and_bad_colors() {
        let err = Palette::from_toml("[colors]\nbackground = \"#000\"\ntext = \"#fff\"", "x");
        assert!(err.unwrap_err().to_string().contains("accent"));
        let err = Palette::from_toml(&MINIMAL.replace("#7e9cd8", "blue"), "x");
        assert!(err.unwrap_err().to_string().contains("invalid color"));
        let err = Palette::from_toml(&format!("{MINIMAL}\nbackgroud = \"#000\""), "x");
        assert!(err.unwrap_err().to_string().contains("backgroud"));
    }

    #[test]
    fn toml_round_trips() {
        let p = Palette::from_toml(MINIMAL, "custom").unwrap();
        assert_eq!(Palette::from_toml(&p.to_toml(), "other").unwrap(), p);
    }

    #[test]
    fn with_accent_keeps_text_readable() {
        let p = Palette::from_toml(MINIMAL, "x")
            .unwrap()
            .with_accent(Color::rgb(0x00, 0x3a, 0x8c));
        assert_eq!(p.accent, Color::rgb(0x00, 0x3a, 0x8c));
        assert_eq!(p.on_accent, WHITE);
    }
}
