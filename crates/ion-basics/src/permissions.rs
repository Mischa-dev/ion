//! Permission prompts: what a site is asking for, in words people read.

/// What a page asked to use, mirroring `WebEnginePermission.PermissionType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Microphone,
    Camera,
    CameraAndMicrophone,
    ScreenShare,
    ScreenShareWithAudio,
    PointerLock,
    Notifications,
    Location,
    Clipboard,
    LocalFonts,
}

impl Kind {
    /// From `QWebEnginePermission::PermissionType`'s integer value. `None` for
    /// `Unsupported` and anything newer than this build knows about; such
    /// requests are denied without a prompt.
    pub fn from_qt(value: i32) -> Option<Kind> {
        Some(match value {
            1 => Kind::Microphone,
            2 => Kind::Camera,
            3 => Kind::CameraAndMicrophone,
            4 => Kind::ScreenShare,
            5 => Kind::ScreenShareWithAudio,
            6 => Kind::PointerLock,
            7 => Kind::Notifications,
            8 => Kind::Location,
            9 => Kind::Clipboard,
            10 => Kind::LocalFonts,
            _ => return None,
        })
    }

    /// What the site wants, finishing the sentence "example.com wants to …".
    pub fn request_phrase(self) -> &'static str {
        match self {
            Kind::Microphone => "use your microphone",
            Kind::Camera => "use your camera",
            Kind::CameraAndMicrophone => "use your camera and microphone",
            Kind::ScreenShare => "see your screen",
            Kind::ScreenShareWithAudio => "see your screen and hear its audio",
            Kind::PointerLock => "hide and lock your mouse pointer",
            Kind::Notifications => "show notifications",
            Kind::Location => "know your location",
            Kind::Clipboard => "read and change your clipboard",
            Kind::LocalFonts => "use the fonts installed on this computer",
        }
    }

    /// A single glyph for the prompt until the icon set lands.
    pub fn glyph(self) -> &'static str {
        match self {
            Kind::Microphone => "🎙",
            Kind::Camera | Kind::CameraAndMicrophone => "📷",
            Kind::ScreenShare | Kind::ScreenShareWithAudio => "🖥",
            Kind::PointerLock => "🖱",
            Kind::Notifications => "🔔",
            Kind::Location => "📍",
            Kind::Clipboard => "📋",
            Kind::LocalFonts => "🔤",
        }
    }
}

/// The prompt's sentence: "example.com wants to use your camera". Falls back
/// to "This page" for origins without a host (local files).
pub fn prompt_text(kind: Kind, origin: &str) -> String {
    let who = crate::display_host(origin).unwrap_or_else(|| "This page".to_owned());
    format!("{who} wants to {}", kind.request_phrase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_qt_matches_the_engine_enum() {
        assert_eq!(Kind::from_qt(0), None);
        assert_eq!(Kind::from_qt(1), Some(Kind::Microphone));
        assert_eq!(Kind::from_qt(3), Some(Kind::CameraAndMicrophone));
        assert_eq!(Kind::from_qt(7), Some(Kind::Notifications));
        assert_eq!(Kind::from_qt(8), Some(Kind::Location));
        assert_eq!(Kind::from_qt(10), Some(Kind::LocalFonts));
        assert_eq!(Kind::from_qt(11), None);
        assert_eq!(Kind::from_qt(-1), None);
    }

    #[test]
    fn prompt_names_the_site() {
        assert_eq!(
            prompt_text(Kind::Camera, "https://meet.example.com"),
            "meet.example.com wants to use your camera"
        );
        assert_eq!(
            prompt_text(Kind::Location, "https://www.maps.example/"),
            "maps.example wants to know your location"
        );
    }

    #[test]
    fn prompt_without_a_host() {
        assert_eq!(
            prompt_text(Kind::Notifications, "file:///home/me/page.html"),
            "This page wants to show notifications"
        );
    }
}
