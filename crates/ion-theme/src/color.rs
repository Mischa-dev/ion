//! sRGB colors as they appear in palette files (`#rrggbb` / `#rrggbbaa`).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An 8-bit sRGB color with alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub const BLACK: Color = Color::rgb(0, 0, 0);
pub const WHITE: Color = Color::rgb(0xff, 0xff, 0xff);

/// Why a color string could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseColorError(String);

impl fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid color {:?}: expected #rgb, #rrggbb or #rrggbbaa",
            self.0
        )
    }
}

impl std::error::Error for ParseColorError {}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 0xff }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Linear interpolation towards `other`: `t = 0` is `self`, `t = 1` is `other`.
    pub fn mix(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
        Color {
            r: lerp(self.r, other.r),
            g: lerp(self.g, other.g),
            b: lerp(self.b, other.b),
            a: lerp(self.a, other.a),
        }
    }

    /// WCAG relative luminance, 0 (black) to 1 (white). Alpha is ignored.
    pub fn luminance(self) -> f32 {
        let channel = |c: u8| {
            let c = f32::from(c) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    /// WCAG contrast ratio between two colors, 1 to 21.
    pub fn contrast(self, other: Color) -> f32 {
        let (a, b) = (self.luminance(), other.luminance());
        let (hi, lo) = if a > b { (a, b) } else { (b, a) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// True when white text reads better on this color than black text.
    pub fn is_dark(self) -> bool {
        self.contrast(WHITE) > self.contrast(BLACK)
    }

    /// Black or white, whichever reads better on top of this color.
    pub fn readable_text(self) -> Color {
        if self.is_dark() { WHITE } else { BLACK }
    }
}

impl FromStr for Color {
    type Err = ParseColorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseColorError(s.to_owned());
        let hex = s.trim();
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if !hex.is_ascii() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(err());
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| err());
        let nibble = |i: usize| u8::from_str_radix(&hex[i..=i], 16).map(|n| n * 0x11);
        match hex.len() {
            3 => Ok(Color::rgb(
                nibble(0).map_err(|_| err())?,
                nibble(1).map_err(|_| err())?,
                nibble(2).map_err(|_| err())?,
            )),
            6 => Ok(Color::rgb(byte(0)?, byte(2)?, byte(4)?)),
            8 => Ok(Color::rgba(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => Err(err()),
        }
    }
}

impl fmt::Display for Color {
    /// `#rrggbb`, or `#rrggbbaa` when not fully opaque.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)?;
        if self.a != 0xff {
            write!(f, "{:02x}", self.a)?;
        }
        Ok(())
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_forms() {
        assert_eq!("#7e9cd8".parse(), Ok(Color::rgb(0x7e, 0x9c, 0xd8)));
        assert_eq!("7E9CD8".parse(), Ok(Color::rgb(0x7e, 0x9c, 0xd8)));
        assert_eq!("#fa0".parse(), Ok(Color::rgb(0xff, 0xaa, 0x00)));
        assert_eq!("#11223380".parse(), Ok(Color::rgba(0x11, 0x22, 0x33, 0x80)));
        assert_eq!(" #000000 ".parse(), Ok(BLACK));
    }

    #[test]
    fn rejects_garbage() {
        for bad in ["", "#", "#12345", "#gggggg", "red", "#1234567", "#ééé"] {
            assert!(bad.parse::<Color>().is_err(), "{bad:?} should not parse");
        }
    }

    #[test]
    fn displays_round_trip() {
        for s in ["#7e9cd8", "#11223380", "#000000"] {
            assert_eq!(s.parse::<Color>().unwrap().to_string(), s);
        }
    }

    #[test]
    fn contrast_matches_wcag() {
        assert!((BLACK.contrast(WHITE) - 21.0).abs() < 0.01);
        assert!((WHITE.contrast(WHITE) - 1.0).abs() < 0.01);
    }

    #[test]
    fn mix_interpolates() {
        assert_eq!(BLACK.mix(WHITE, 0.0), BLACK);
        assert_eq!(BLACK.mix(WHITE, 1.0), WHITE);
        assert_eq!(BLACK.mix(WHITE, 0.5), Color::rgb(128, 128, 128));
    }

    #[test]
    fn readable_text_picks_contrasting_color() {
        assert_eq!(Color::rgb(0x16, 0x16, 0x1d).readable_text(), WHITE);
        assert_eq!(Color::rgb(0xf2, 0xf2, 0xf2).readable_text(), BLACK);
    }
}
