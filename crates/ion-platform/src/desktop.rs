//! Desktop integration that has to be set up before Qt starts.
//!
//! On Linux, Qt only asks xdg-desktop-portal for file pickers when its
//! `xdgdesktopportal` platform theme is in use, which happens by default only
//! inside Flatpak. Without it, "Upload" and "Save as" get Qt's own dialog
//! instead of the desktop's, and miss its recent files, bookmarks and
//! sandbox-friendly access. Ion picks that theme unless one is already set
//! (KDE sets its own, and people choose `qt6ct` or `gtk3`).

use crate::chromium::Os;

/// Environment variable Qt reads its platform theme from.
pub const PLATFORM_THEME_VAR: &str = "QT_QPA_PLATFORMTHEME";

/// The platform theme to set, given the current value, or `None` to leave
/// it alone.
pub fn platform_theme(os: Os, current: Option<&str>) -> Option<&'static str> {
    let unset = current.is_none_or(|value| value.trim().is_empty());
    (os == Os::Linux && unset).then_some("xdgdesktopportal")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_uses_the_portal_unless_a_theme_is_set() {
        assert_eq!(platform_theme(Os::Linux, None), Some("xdgdesktopportal"));
        assert_eq!(
            platform_theme(Os::Linux, Some(" ")),
            Some("xdgdesktopportal")
        );
        assert_eq!(platform_theme(Os::Linux, Some("qt6ct")), None);
        assert_eq!(platform_theme(Os::MacOs, None), None);
    }
}
