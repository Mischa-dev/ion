//! Theme sources: where the active palette comes from.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::builtin;
use crate::color::Color;
use crate::palette::Palette;

/// Theme id meaning "Ion's built-in light or dark theme, following the system".
pub const AUTO: &str = "auto";

/// Where the palette comes from, chosen by `theme.source` in config.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThemeSource {
    /// A built-in theme, or a palette file installed in `~/.config/ion/themes`,
    /// by id. [`AUTO`] picks Ion's light or dark theme to match the system.
    Builtin(String),
    /// DankMaterialShell: a palette file written by Ion's matugen template.
    Dms(PathBuf),
    /// Ion's light or dark theme to match the system, with the system accent.
    System,
    /// A palette file the person maintains by hand or through Nix.
    Manual(PathBuf),
}

/// What the desktop reports about its look. Supplied by the UI layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SystemAppearance {
    pub dark: bool,
    pub accent: Option<Color>,
}

/// Why no palette could be produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeError(String);

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ThemeError {}

/// Ion's config directory and the theme files in it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dirs {
    /// The same directory `ion-config` uses: `$ION_CONFIG_DIR`, else
    /// `$XDG_CONFIG_HOME/ion`, else `~/.config/ion` (on macOS too).
    pub config: Option<PathBuf>,
}

impl Dirs {
    pub fn from_env() -> Self {
        let var = |name: &str| {
            std::env::var_os(name)
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
        };
        let config = var("ION_CONFIG_DIR")
            .or_else(|| var("XDG_CONFIG_HOME").map(|d| d.join("ion")))
            .or_else(|| home().map(|h| h.join(".config").join("ion")));
        Self { config }
    }

    /// Community themes: `<config>/themes/<id>.toml`.
    pub fn themes(&self) -> Option<PathBuf> {
        self.config.as_ref().map(|c| c.join("themes"))
    }

    /// Where Ion's matugen template writes by default.
    pub fn dms_palette(&self) -> Option<PathBuf> {
        self.config.as_ref().map(|c| c.join("dms-palette.toml"))
    }

    /// The default palette file for the manual source.
    pub fn manual_palette(&self) -> Option<PathBuf> {
        self.config.as_ref().map(|c| c.join("palette.toml"))
    }

    /// Ids of every selectable theme: built-ins first, then installed files.
    pub fn theme_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = builtin::names().map(str::to_owned).collect();
        let mut installed: Vec<String> = self
            .themes()
            .and_then(|dir| std::fs::read_dir(dir).ok())
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|p| p.extension().is_some_and(|e| e == "toml"))
            .filter_map(|p| p.file_stem()?.to_str().map(str::to_owned))
            .filter(|id| !ids.contains(id))
            .collect();
        installed.sort();
        ids.extend(installed);
        ids
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
}

/// Expand a leading `~/` the way config files and Nix options spell paths.
fn expand(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), home()) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(path),
    }
}

impl ThemeSource {
    /// Build a source from config values. `kind` is `builtin`, `dms`, `system`
    /// or `manual`; `theme` is the theme id for `builtin` (empty means
    /// [`AUTO`]); `path` overrides the palette file for `dms` and `manual`.
    pub fn from_settings(
        kind: &str,
        theme: &str,
        path: &str,
        dirs: &Dirs,
    ) -> Result<Self, ThemeError> {
        let file = |default: Option<PathBuf>| {
            if path.trim().is_empty() {
                default.ok_or_else(|| ThemeError("no config directory: set a palette path".into()))
            } else {
                Ok(expand(path.trim()))
            }
        };
        match kind.trim().to_ascii_lowercase().as_str() {
            "" | "builtin" => {
                let theme = theme.trim();
                Ok(Self::Builtin(
                    if theme.is_empty() { AUTO } else { theme }.to_owned(),
                ))
            }
            "dms" => Ok(Self::Dms(file(dirs.dms_palette())?)),
            "system" => Ok(Self::System),
            "manual" => Ok(Self::Manual(file(dirs.manual_palette())?)),
            other => Err(ThemeError(format!(
                "unknown theme source {other:?}: expected builtin, dms, system or manual"
            ))),
        }
    }

    /// The file this source reads, which the app watches for live reload.
    pub fn palette_file(&self, dirs: &Dirs) -> Option<PathBuf> {
        match self {
            Self::Dms(path) | Self::Manual(path) => Some(path.clone()),
            Self::Builtin(id) if id != AUTO && builtin::get(id).is_none() => {
                dirs.themes().map(|d| d.join(format!("{id}.toml")))
            }
            Self::Builtin(_) | Self::System => None,
        }
    }

    /// Produce the palette for this source.
    pub fn resolve(&self, system: &SystemAppearance, dirs: &Dirs) -> Result<Palette, ThemeError> {
        let follow_system = || {
            builtin::get(if system.dark {
                builtin::DEFAULT_DARK
            } else {
                builtin::DEFAULT_LIGHT
            })
            .expect("default themes are built in")
        };
        match self {
            Self::Builtin(id) if id == AUTO => Ok(follow_system()),
            Self::Builtin(id) => match builtin::get(id) {
                Some(palette) => Ok(palette),
                None => match self.palette_file(dirs) {
                    Some(path) if path.exists() => read_palette(&path, id),
                    _ => Err(ThemeError(format!("no theme named {id:?}"))),
                },
            },
            Self::System => Ok(match system.accent {
                Some(accent) => follow_system().with_accent(accent),
                None => follow_system(),
            }),
            Self::Dms(path) | Self::Manual(path) => read_palette(path, "Custom"),
        }
    }
}

fn read_palette(path: &Path, fallback_name: &str) -> Result<Palette, ThemeError> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| ThemeError(format!("{}: {e}", path.display())))?;
    Palette::from_toml(&src, fallback_name)
        .map_err(|e| ThemeError(format!("{}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::Scheme;
    use crate::test_dir;

    fn dirs(config: &Path) -> Dirs {
        Dirs {
            config: Some(config.to_owned()),
        }
    }

    #[test]
    fn auto_follows_system_mode() {
        let d = Dirs::default();
        let auto = ThemeSource::Builtin(AUTO.into());
        let dark = SystemAppearance {
            dark: true,
            accent: None,
        };
        assert_eq!(auto.resolve(&dark, &d).unwrap().scheme, Scheme::Dark);
        let light = SystemAppearance::default();
        assert_eq!(auto.resolve(&light, &d).unwrap().scheme, Scheme::Light);
    }

    #[test]
    fn system_uses_system_accent() {
        let accent = Color::rgb(0xff, 0x95, 0x00);
        let system = SystemAppearance {
            dark: true,
            accent: Some(accent),
        };
        let p = ThemeSource::System
            .resolve(&system, &Dirs::default())
            .unwrap();
        assert_eq!(p.accent, accent);
        assert_eq!(p.scheme, Scheme::Dark);
    }

    #[test]
    fn parses_settings() {
        let d = dirs(Path::new("/cfg/ion"));
        let parse = |k, t, p| ThemeSource::from_settings(k, t, p, &d);
        assert_eq!(parse("", "", ""), Ok(ThemeSource::Builtin(AUTO.into())));
        assert_eq!(
            parse("builtin", "nord", ""),
            Ok(ThemeSource::Builtin("nord".into()))
        );
        assert_eq!(
            parse("DMS", "", ""),
            Ok(ThemeSource::Dms("/cfg/ion/dms-palette.toml".into()))
        );
        assert_eq!(
            parse("manual", "", "/etc/ion.toml"),
            Ok(ThemeSource::Manual("/etc/ion.toml".into()))
        );
        assert_eq!(parse("system", "", ""), Ok(ThemeSource::System));
        assert!(parse("gtk", "", "").is_err());
    }

    #[test]
    fn reads_palette_files_and_installed_themes() {
        let dir = test_dir("source");
        std::fs::create_dir_all(dir.join("themes")).unwrap();
        std::fs::write(
            dir.join("themes/paper.toml"),
            "[colors]\nbackground = \"#ffffff\"\ntext = \"#111111\"\naccent = \"#0055cc\"",
        )
        .unwrap();
        let d = dirs(&dir);
        let system = SystemAppearance::default();

        assert!(d.theme_ids().contains(&"paper".to_owned()));
        assert_eq!(d.theme_ids()[0], builtin::names().next().unwrap());

        let paper = ThemeSource::Builtin("paper".into());
        assert_eq!(paper.palette_file(&d), Some(dir.join("themes/paper.toml")));
        let p = paper.resolve(&system, &d).unwrap();
        assert_eq!(p.name, "paper");
        assert_eq!(p.scheme, Scheme::Light);

        let manual = ThemeSource::Manual(dir.join("themes/paper.toml"));
        assert_eq!(
            manual.resolve(&system, &d).unwrap().background,
            p.background
        );

        let missing = ThemeSource::Dms(dir.join("nope.toml"));
        assert!(missing.resolve(&system, &d).is_err());
        let unknown = ThemeSource::Builtin("nope".into());
        assert!(unknown.resolve(&system, &d).is_err());
        assert_eq!(ThemeSource::Builtin("nord".into()).palette_file(&d), None);

        std::fs::remove_dir_all(dir).unwrap();
    }
}
