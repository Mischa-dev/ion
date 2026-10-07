//! How web pages follow the theme (`theme.pages` in the config).

use std::str::FromStr;

use crate::palette::Scheme;

/// What pages are told about light and dark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PageTheming {
    /// Pages see the theme's scheme as `prefers-color-scheme`, so a dark Ion
    /// theme gets the dark version of sites that have one.
    #[default]
    Match,
    /// Pages see the system's light/dark setting, whatever Ion's theme is.
    System,
    /// Like `Match`, and with a dark theme, pages that have no dark style of
    /// their own are darkened too.
    Darken,
}

impl PageTheming {
    pub const NAMES: [&str; 3] = ["match", "system", "darken"];

    /// The scheme pages should see with a theme of `scheme`; `None` leaves
    /// the system's.
    pub fn scheme(self, scheme: Scheme) -> Option<Scheme> {
        match self {
            PageTheming::Match | PageTheming::Darken => Some(scheme),
            PageTheming::System => None,
        }
    }

    /// Whether pages without a dark style get darkened.
    pub fn darken(self, scheme: Scheme) -> bool {
        self == PageTheming::Darken && scheme == Scheme::Dark
    }
}

impl FromStr for PageTheming {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "" | "match" => Ok(PageTheming::Match),
            "system" => Ok(PageTheming::System),
            "darken" => Ok(PageTheming::Darken),
            other => Err(format!(
                "unknown theme.pages {other:?}; expected one of {}",
                PageTheming::NAMES.join(", ")
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names() {
        for name in PageTheming::NAMES {
            assert!(name.parse::<PageTheming>().is_ok(), "{name}");
        }
        assert_eq!("".parse(), Ok(PageTheming::Match));
        assert!(
            "dark"
                .parse::<PageTheming>()
                .unwrap_err()
                .contains("darken")
        );
    }

    #[test]
    fn scheme_and_darkening() {
        use PageTheming::*;
        assert_eq!(Match.scheme(Scheme::Light), Some(Scheme::Light));
        assert_eq!(System.scheme(Scheme::Dark), None);
        assert_eq!(Darken.scheme(Scheme::Dark), Some(Scheme::Dark));
        assert!(Darken.darken(Scheme::Dark));
        assert!(!Darken.darken(Scheme::Light));
        assert!(!Match.darken(Scheme::Dark));
    }
}
