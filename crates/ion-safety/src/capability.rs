//! What can be asked for: site capabilities, agent actions and their tiers.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Something a page asks to use, mirroring `QWebEnginePermission::PermissionType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SiteCapability {
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

impl SiteCapability {
    pub const ALL: [SiteCapability; 10] = [
        SiteCapability::Microphone,
        SiteCapability::Camera,
        SiteCapability::CameraAndMicrophone,
        SiteCapability::ScreenShare,
        SiteCapability::ScreenShareWithAudio,
        SiteCapability::PointerLock,
        SiteCapability::Notifications,
        SiteCapability::Location,
        SiteCapability::Clipboard,
        SiteCapability::LocalFonts,
    ];

    /// From `QWebEnginePermission::PermissionType`'s integer value. `None` for
    /// `Unsupported` and types newer than this build; deny those.
    pub fn from_qt(value: i32) -> Option<SiteCapability> {
        let index = usize::try_from(value).ok()?.checked_sub(1)?;
        SiteCapability::ALL.get(index).copied()
    }

    /// The `QWebEnginePermission::PermissionType` integer value.
    pub fn to_qt(self) -> i32 {
        SiteCapability::ALL
            .iter()
            .position(|c| *c == self)
            .map_or(0, |i| i as i32 + 1)
    }

    /// Finishes "example.com wants to …".
    pub fn request_phrase(self) -> &'static str {
        match self {
            SiteCapability::Microphone => "use your microphone",
            SiteCapability::Camera => "use your camera",
            SiteCapability::CameraAndMicrophone => "use your camera and microphone",
            SiteCapability::ScreenShare => "see your screen",
            SiteCapability::ScreenShareWithAudio => "see your screen and hear its audio",
            SiteCapability::PointerLock => "hide and lock your mouse pointer",
            SiteCapability::Notifications => "show notifications",
            SiteCapability::Location => "know your location",
            SiteCapability::Clipboard => "read and change your clipboard",
            SiteCapability::LocalFonts => "use the fonts installed on this computer",
        }
    }

    /// A short name for settings lists: "Camera", "Location".
    pub fn label(self) -> &'static str {
        match self {
            SiteCapability::Microphone => "Microphone",
            SiteCapability::Camera => "Camera",
            SiteCapability::CameraAndMicrophone => "Camera and microphone",
            SiteCapability::ScreenShare => "Screen sharing",
            SiteCapability::ScreenShareWithAudio => "Screen and audio sharing",
            SiteCapability::PointerLock => "Pointer lock",
            SiteCapability::Notifications => "Notifications",
            SiteCapability::Location => "Location",
            SiteCapability::Clipboard => "Clipboard",
            SiteCapability::LocalFonts => "Local fonts",
        }
    }
}

/// How careful Ion is with an agent action. Ordered: an action of a higher
/// tier needs at least as much trust as one below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    /// Looking: page text, accessibility tree, screenshots, tab lists.
    Read,
    /// Doing things that are easy to undo: clicking, typing, navigating.
    Act,
    /// Sending something out: submit, post, send.
    Commit,
    /// Handing over the person's own data: personal details, local files.
    Personal,
    /// Spending money.
    Purchase,
    /// Using a connector (an MCP server or other outside tool).
    Connector,
    /// Never allowed.
    Forbidden,
}

impl Tier {
    /// Whether this is one of the ordered tiers that apply to sites
    /// (read through purchase).
    pub fn is_site_tier(self) -> bool {
        self <= Tier::Purchase
    }

    pub fn id(self) -> &'static str {
        match self {
            Tier::Read => "read",
            Tier::Act => "act",
            Tier::Commit => "commit",
            Tier::Personal => "personal",
            Tier::Purchase => "purchase",
            Tier::Connector => "connector",
            Tier::Forbidden => "forbidden",
        }
    }

    pub fn from_id(id: &str) -> Option<Tier> {
        [
            Tier::Read,
            Tier::Act,
            Tier::Commit,
            Tier::Personal,
            Tier::Purchase,
            Tier::Connector,
            Tier::Forbidden,
        ]
        .into_iter()
        .find(|t| t.id() == id)
    }
}

/// Something an agent wants to do.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum Action {
    ReadPage,
    ListTabs,
    Navigate,
    OpenTab,
    CloseTab,
    Interact,
    DevTools,
    Download,
    Submit,
    UploadFile,
    EnterPersonalData,
    Purchase,
    /// Use the named connector.
    UseConnector(String),
    /// Saved passwords, payment details, cookies, tokens. A hard limit:
    /// never allowed.
    ReadCredentials,
}

const SIMPLE_ACTIONS: [(Action, &str); 13] = [
    (Action::ReadPage, "readPage"),
    (Action::ListTabs, "listTabs"),
    (Action::Navigate, "navigate"),
    (Action::OpenTab, "openTab"),
    (Action::CloseTab, "closeTab"),
    (Action::Interact, "interact"),
    (Action::DevTools, "devTools"),
    (Action::Download, "download"),
    (Action::Submit, "submit"),
    (Action::UploadFile, "uploadFile"),
    (Action::EnterPersonalData, "enterPersonalData"),
    (Action::Purchase, "purchase"),
    (Action::ReadCredentials, "readCredentials"),
];

const CONNECTOR_PREFIX: &str = "useConnector:";

impl Action {
    pub fn tier(&self) -> Tier {
        match self {
            Action::ReadPage | Action::ListTabs => Tier::Read,
            Action::Navigate
            | Action::OpenTab
            | Action::CloseTab
            | Action::Interact
            | Action::DevTools
            | Action::Download => Tier::Act,
            Action::Submit => Tier::Commit,
            Action::UploadFile | Action::EnterPersonalData => Tier::Personal,
            Action::Purchase => Tier::Purchase,
            Action::UseConnector(_) => Tier::Connector,
            Action::ReadCredentials => Tier::Forbidden,
        }
    }

    /// The id used in config and the log: `"submit"`, `"useConnector:github"`.
    pub fn id(&self) -> String {
        match self {
            Action::UseConnector(name) => format!("{CONNECTOR_PREFIX}{name}"),
            other => SIMPLE_ACTIONS
                .iter()
                .find(|(a, _)| a == other)
                .map(|(_, id)| (*id).to_owned())
                .unwrap_or_default(),
        }
    }

    pub fn from_id(id: &str) -> Option<Action> {
        if let Some(name) = id.strip_prefix(CONNECTOR_PREFIX) {
            return (!name.is_empty()).then(|| Action::UseConnector(name.to_owned()));
        }
        SIMPLE_ACTIONS
            .iter()
            .find(|(_, i)| *i == id)
            .map(|(a, _)| a.clone())
    }

    /// Whether the action is about a site and so needs one in its request.
    pub fn needs_site(&self) -> bool {
        !matches!(self, Action::ListTabs | Action::UseConnector(_))
    }

    /// Whether the action works on an existing tab and so needs one in its
    /// request (so taking a tab back can't be sidestepped by leaving it out).
    pub fn needs_tab(&self) -> bool {
        !matches!(
            self,
            Action::ListTabs | Action::OpenTab | Action::UseConnector(_) | Action::ReadCredentials
        )
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.id())
    }
}

impl From<Action> for String {
    fn from(action: Action) -> String {
        action.id()
    }
}

impl TryFrom<String> for Action {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Action::from_id(&value).ok_or_else(|| format!("unknown action: {value:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_capabilities_match_the_engine_enum() {
        assert_eq!(SiteCapability::from_qt(0), None);
        assert_eq!(SiteCapability::from_qt(1), Some(SiteCapability::Microphone));
        assert_eq!(
            SiteCapability::from_qt(3),
            Some(SiteCapability::CameraAndMicrophone)
        );
        assert_eq!(
            SiteCapability::from_qt(7),
            Some(SiteCapability::Notifications)
        );
        assert_eq!(SiteCapability::from_qt(8), Some(SiteCapability::Location));
        assert_eq!(
            SiteCapability::from_qt(10),
            Some(SiteCapability::LocalFonts)
        );
        assert_eq!(SiteCapability::from_qt(11), None);
        assert_eq!(SiteCapability::from_qt(-1), None);
        for cap in SiteCapability::ALL {
            assert_eq!(SiteCapability::from_qt(cap.to_qt()), Some(cap));
        }
    }

    #[test]
    fn tiers_are_ordered_by_risk() {
        assert!(Tier::Read < Tier::Act);
        assert!(Tier::Act < Tier::Commit);
        assert!(Tier::Commit < Tier::Personal);
        assert!(Tier::Personal < Tier::Purchase);
        assert!(Tier::Purchase.is_site_tier());
        assert!(!Tier::Connector.is_site_tier());
        assert!(!Tier::Forbidden.is_site_tier());
        for tier in [
            "read",
            "act",
            "commit",
            "personal",
            "purchase",
            "connector",
            "forbidden",
        ] {
            assert_eq!(Tier::from_id(tier).unwrap().id(), tier);
        }
        assert_eq!(Tier::from_id("nope"), None);
    }

    #[test]
    fn action_ids_round_trip() {
        for (action, id) in SIMPLE_ACTIONS {
            assert_eq!(action.id(), id);
            assert_eq!(Action::from_id(id), Some(action));
        }
        let github = Action::UseConnector("github".into());
        assert_eq!(github.id(), "useConnector:github");
        assert_eq!(Action::from_id("useConnector:github"), Some(github));
        assert_eq!(Action::from_id("useConnector:"), None);
        assert_eq!(Action::from_id("hack"), None);
    }

    #[test]
    fn action_tiers() {
        assert_eq!(Action::ReadPage.tier(), Tier::Read);
        assert_eq!(Action::Interact.tier(), Tier::Act);
        assert_eq!(Action::Submit.tier(), Tier::Commit);
        assert_eq!(Action::EnterPersonalData.tier(), Tier::Personal);
        assert_eq!(Action::Purchase.tier(), Tier::Purchase);
        assert_eq!(Action::UseConnector("x".into()).tier(), Tier::Connector);
        assert_eq!(Action::ReadCredentials.tier(), Tier::Forbidden);
        assert!(!Action::ListTabs.needs_site());
        assert!(Action::Navigate.needs_site());
        assert!(Action::Interact.needs_tab());
        assert!(Action::Submit.needs_tab());
        assert!(!Action::OpenTab.needs_tab());
        assert!(!Action::ListTabs.needs_tab());
    }
}
