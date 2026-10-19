//! Chromium switches for QtWebEngine.
//!
//! QtWebEngine reads extra Chromium switches from `QTWEBENGINE_CHROMIUM_FLAGS`.
//! Ion adds the features that turn on hardware video decoding and PipeWire
//! screen capture on Linux, and the Widevine CDM's location when the package
//! ships one, and merges them with whatever the user already put there, so a
//! user's own `--enable-features` or `--disable-features` keeps working:
//! Chromium only honours the last copy of each switch, so they cannot simply
//! be appended.

/// Environment variable QtWebEngine reads its Chromium switches from.
pub const FLAGS_VAR: &str = "QTWEBENGINE_CHROMIUM_FLAGS";

/// Set to `0` to keep Ion from enabling hardware video decoding.
pub const HARDWARE_VIDEO_VAR: &str = "ION_HARDWARE_VIDEO";

/// Path to `libwidevinecdm.so`, the DRM module Netflix, Spotify and other
/// streaming sites need. The Nix package's `ion-widevine` variant sets it.
pub const WIDEVINE_VAR: &str = "ION_WIDEVINE_PATH";

/// Screen sharing on Wayland goes through xdg-desktop-portal and PipeWire;
/// X11's capturer sees nothing there. On by default in recent Chromium, but
/// listing it keeps it on whatever QtWebEngine ships.
const LINUX_FEATURES: &[&str] = &["WebRTCPipeWireCapturer"];

/// VA-API video decoding (and encoding, for WebRTC) on Linux. Chromium renamed
/// the decode feature over time; listing every name is harmless because unknown
/// features are ignored, and keeps this working across QtWebEngine updates.
/// `VaapiIgnoreDriverChecks` lets drivers other than Intel's (Mesa on AMD,
/// nvidia-vaapi-driver) be used.
const LINUX_VIDEO_FEATURES: &[&str] = &[
    "AcceleratedVideoDecodeLinuxGL",
    "VaapiVideoDecodeLinuxGL",
    "VaapiVideoDecoder",
    "AcceleratedVideoEncoder",
    "VaapiVideoEncoder",
    "VaapiIgnoreDriverChecks",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    MacOs,
    Other,
}

impl Os {
    pub fn current() -> Self {
        if cfg!(target_os = "linux") {
            Os::Linux
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Other
        }
    }
}

/// What Ion wants from Chromium on this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub os: Os,
    pub hardware_video: bool,
    /// Where the Widevine CDM is, if Ion knows.
    pub widevine: Option<String>,
}

impl Options {
    /// Options for the running system, honouring [`HARDWARE_VIDEO_VAR`]
    /// and [`WIDEVINE_VAR`].
    pub fn from_env() -> Self {
        let hardware_video = std::env::var(HARDWARE_VIDEO_VAR).map_or(true, |v| v.trim() != "0");
        let widevine = std::env::var(WIDEVINE_VAR)
            .ok()
            .map(|path| path.trim().to_owned())
            .filter(|path| !path.is_empty() && std::path::Path::new(path).is_file());
        Self {
            os: Os::current(),
            hardware_video,
            widevine,
        }
    }

    /// Features Ion turns on. macOS decodes through VideoToolbox and
    /// captures screens through ScreenCaptureKit by default.
    fn features(&self) -> Vec<&'static str> {
        let mut features = Vec::new();
        if self.os == Os::Linux {
            features.extend_from_slice(LINUX_FEATURES);
            if self.hardware_video {
                features.extend_from_slice(LINUX_VIDEO_FEATURES);
            }
        }
        features
    }
}

/// The value to store in [`FLAGS_VAR`], given the user's current value.
/// Returns `None` when there is nothing to change.
pub fn merged_flags(options: &Options, user: Option<&str>) -> Option<String> {
    let ours = options.features();
    // The user's own `--widevine-path` wins.
    let widevine = options.widevine.as_deref().filter(|_| {
        !user
            .unwrap_or_default()
            .split_whitespace()
            .any(|arg| arg.starts_with("--widevine-path="))
    });
    if ours.is_empty() && widevine.is_none() {
        return None;
    }

    let mut other = Vec::new();
    let mut enabled: Vec<String> = Vec::new();
    let mut disabled: Vec<String> = Vec::new();
    for arg in user.unwrap_or_default().split_whitespace() {
        if let Some(list) = arg.strip_prefix("--enable-features=") {
            push_features(&mut enabled, list);
        } else if let Some(list) = arg.strip_prefix("--disable-features=") {
            push_features(&mut disabled, list);
        } else {
            other.push(arg.to_owned());
        }
    }

    // The user's own `--disable-features` wins over Ion's defaults.
    for feature in ours {
        if !disabled.iter().any(|f| f == feature) && !enabled.iter().any(|f| f == feature) {
            enabled.push(feature.to_owned());
        }
    }

    let mut out = other;
    if let Some(path) = widevine {
        out.push(format!("--widevine-path={path}"));
    }
    if !enabled.is_empty() {
        out.push(format!("--enable-features={}", enabled.join(",")));
    }
    if !disabled.is_empty() {
        out.push(format!("--disable-features={}", disabled.join(",")));
    }
    Some(out.join(" "))
}

fn push_features(into: &mut Vec<String>, list: &str) {
    for feature in list.split(',').map(str::trim).filter(|f| !f.is_empty()) {
        if !into.iter().any(|f| f == feature) {
            into.push(feature.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINUX: Options = Options {
        os: Os::Linux,
        hardware_video: true,
        widevine: None,
    };

    #[test]
    fn linux_enables_vaapi_and_pipewire() {
        let flags = merged_flags(&LINUX, None).unwrap();
        assert!(flags.starts_with("--enable-features=WebRTCPipeWireCapturer,"));
        assert!(flags.contains("AcceleratedVideoDecodeLinuxGL"));
        assert!(flags.contains("VaapiIgnoreDriverChecks"));
        assert!(!flags.contains("--disable-features"));
    }

    #[test]
    fn macos_and_opt_out_leave_video_alone() {
        let mac = Options {
            os: Os::MacOs,
            ..LINUX
        };
        assert_eq!(merged_flags(&mac, Some("--foo")), None);
        let off = Options {
            hardware_video: false,
            ..LINUX
        };
        assert_eq!(
            merged_flags(&off, Some("--foo")).unwrap(),
            "--foo --enable-features=WebRTCPipeWireCapturer"
        );
    }

    #[test]
    fn widevine_path_is_passed_unless_the_user_set_one() {
        let mac = Options {
            os: Os::MacOs,
            widevine: Some("/nix/store/x/libwidevinecdm.so".to_owned()),
            ..LINUX
        };
        assert_eq!(
            merged_flags(&mac, None).unwrap(),
            "--widevine-path=/nix/store/x/libwidevinecdm.so"
        );
        assert_eq!(merged_flags(&mac, Some("--widevine-path=/mine.so")), None);
        let linux = Options {
            os: Os::Linux,
            ..mac
        };
        assert!(
            merged_flags(&linux, None)
                .unwrap()
                .starts_with("--widevine-path=/nix/store/x/libwidevinecdm.so --enable-features=")
        );
    }

    #[test]
    fn user_features_are_merged_into_one_switch() {
        let flags = merged_flags(
            &LINUX,
            Some("--ignore-gpu-blocklist --enable-features=Foo,VaapiVideoDecoder"),
        )
        .unwrap();
        assert_eq!(flags.matches("--enable-features=").count(), 1);
        assert!(flags.starts_with(
            "--ignore-gpu-blocklist --enable-features=Foo,VaapiVideoDecoder,WebRTCPipeWireCapturer,"
        ));
        assert_eq!(flags.matches("VaapiVideoDecoder").count(), 1);
    }

    #[test]
    fn user_disabled_features_win() {
        let flags = merged_flags(
            &LINUX,
            Some("--disable-features=VaapiIgnoreDriverChecks --disable-features=Bar"),
        )
        .unwrap();
        assert!(flags.ends_with("--disable-features=VaapiIgnoreDriverChecks,Bar"));
        let enabled = flags.split_whitespace().next().unwrap();
        assert!(!enabled.contains("VaapiIgnoreDriverChecks"));
        assert!(enabled.contains("VaapiVideoDecoder"));
    }
}
