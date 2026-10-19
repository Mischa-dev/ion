//! The words on Ion's security key and passkey dialog.
//!
//! QtWebEngine runs the WebAuthn exchange with the authenticator itself and
//! asks the browser only when a person has to do something: pick an account,
//! enter or set the key's PIN, touch the key, or read why it failed. Each of
//! those is a [`State`]; this turns one into a heading and a sentence.

/// Where the exchange is, as `WebEngineWebAuthUxRequest.WebAuthUxState`
/// numbers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Waiting for a key to be plugged in or touched.
    NotStarted,
    SelectAccount,
    CollectPin,
    /// The key wants a touch to finish.
    FinishTokenCollection,
    RequestFailed,
    /// Over: the dialog closes.
    Done,
}

impl State {
    pub fn from_qt(value: i32) -> Self {
        match value {
            1 => Self::SelectAccount,
            2 => Self::CollectPin,
            3 => Self::FinishTokenCollection,
            4 => Self::RequestFailed,
            5 | 6 => Self::Done,
            _ => Self::NotStarted,
        }
    }
}

/// Why the PIN is asked for (`PinEntryReason`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinReason {
    Set,
    Change,
    Challenge,
}

impl PinReason {
    pub fn from_qt(value: i32) -> Self {
        match value {
            0 => Self::Set,
            1 => Self::Change,
            _ => Self::Challenge,
        }
    }
}

/// What was wrong with the last PIN (`PinEntryError`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinError {
    None,
    BiometricsLocked,
    WrongPin,
    TooShort,
    InvalidCharacters,
    SameAsCurrent,
}

impl PinError {
    pub fn from_qt(value: i32) -> Self {
        match value {
            1 => Self::BiometricsLocked,
            2 => Self::WrongPin,
            3 => Self::TooShort,
            4 => Self::InvalidCharacters,
            5 => Self::SameAsCurrent,
            _ => Self::None,
        }
    }
}

/// What the dialog shows for one state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PinDetails {
    pub reason: Option<PinReason>,
    pub error: Option<PinError>,
    pub min_length: i32,
    pub attempts_left: i32,
}

/// The heading for `state` on site `site` (the relying party id).
pub fn heading(state: State, site: &str, pin: &PinDetails) -> String {
    let site = if site.is_empty() { "This site" } else { site };
    match state {
        State::NotStarted => format!("{site} wants to use your security key or passkey"),
        State::SelectAccount => format!("Choose an account for {site}"),
        State::CollectPin => match pin.reason.unwrap_or(PinReason::Challenge) {
            PinReason::Set => "Set a PIN for your security key".to_owned(),
            PinReason::Change => "Change your security key's PIN".to_owned(),
            PinReason::Challenge => "Enter your security key's PIN".to_owned(),
        },
        State::FinishTokenCollection => "Touch your security key".to_owned(),
        State::RequestFailed => "Your security key couldn't be used".to_owned(),
        State::Done => String::new(),
    }
}

/// The sentence under the heading. `failure` is the request's
/// `RequestFailureReason` number, used only for [`State::RequestFailed`].
pub fn detail(state: State, pin: &PinDetails, failure: i32) -> String {
    match state {
        State::NotStarted => "Plug in your key and touch it, or press Cancel.".to_owned(),
        State::SelectAccount => "Your key has more than one account for this site.".to_owned(),
        State::CollectPin => pin_detail(pin),
        State::FinishTokenCollection => "Touch or tap your key again to finish.".to_owned(),
        State::RequestFailed => failure_detail(failure).to_owned(),
        State::Done => String::new(),
    }
}

fn pin_detail(pin: &PinDetails) -> String {
    let tries = |n: i32| match n {
        1 => " 1 try left before the key locks.".to_owned(),
        n if n > 1 => format!(" {n} tries left."),
        _ => String::new(),
    };
    match pin.error.unwrap_or(PinError::None) {
        PinError::WrongPin => format!("That PIN is wrong.{}", tries(pin.attempts_left)),
        PinError::TooShort => format!(
            "The PIN needs at least {} characters.",
            pin.min_length.max(4)
        ),
        PinError::InvalidCharacters => "The PIN has characters the key can't take.".to_owned(),
        PinError::SameAsCurrent => "Pick a PIN different from the current one.".to_owned(),
        PinError::BiometricsLocked => {
            "Fingerprint unlock is locked after too many tries. Enter the PIN instead.".to_owned()
        }
        PinError::None => match pin.reason.unwrap_or(PinReason::Challenge) {
            PinReason::Set | PinReason::Change => format!(
                "Use at least {} characters. You'll need it each time you sign in with this key.",
                pin.min_length.max(4)
            ),
            PinReason::Challenge => format!(
                "The site asks for it to make sure it's you.{}",
                if pin.attempts_left > 0 && pin.attempts_left <= 3 {
                    tries(pin.attempts_left)
                } else {
                    String::new()
                }
            ),
        },
    }
}

/// `RequestFailureReason` in words.
fn failure_detail(reason: i32) -> &'static str {
    match reason {
        0 => "The request timed out before the key answered. Try again.",
        1 => "This key isn't registered with this site. Try another key.",
        2 => "This key is already registered with this site.",
        3 => "The key is locked after too many wrong PINs. Unplug it and plug it back in.",
        4 => {
            "The key is locked after too many wrong PINs. It has to be reset before it works again."
        }
        5 => "The key was removed while entering the PIN.",
        6 => "This key can't store sign-ins on itself, which this site needs.",
        7 => "This key can't check that it's you (no PIN or fingerprint), which this site needs.",
        8 => "This key doesn't support what this site asked for.",
        9 => "This key doesn't support the security this site needs.",
        10 => "This key is full. Remove a sign-in from it and try again.",
        _ => "The request was cancelled.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_map_from_qt() {
        assert_eq!(State::from_qt(0), State::NotStarted);
        assert_eq!(State::from_qt(2), State::CollectPin);
        assert_eq!(State::from_qt(5), State::Done);
        assert_eq!(State::from_qt(6), State::Done);
        assert_eq!(PinReason::from_qt(0), PinReason::Set);
        assert_eq!(PinError::from_qt(2), PinError::WrongPin);
    }

    #[test]
    fn headings_name_the_site() {
        let pin = PinDetails::default();
        assert_eq!(
            heading(State::SelectAccount, "github.com", &pin),
            "Choose an account for github.com"
        );
        assert!(heading(State::NotStarted, "", &pin).starts_with("This site"));
        assert_eq!(heading(State::Done, "a.com", &pin), "");
    }

    #[test]
    fn pin_errors_explain_what_to_do() {
        let wrong = PinDetails {
            reason: Some(PinReason::Challenge),
            error: Some(PinError::WrongPin),
            attempts_left: 2,
            ..PinDetails::default()
        };
        assert_eq!(
            detail(State::CollectPin, &wrong, 0),
            "That PIN is wrong. 2 tries left."
        );
        let short = PinDetails {
            reason: Some(PinReason::Set),
            error: Some(PinError::TooShort),
            min_length: 6,
            ..PinDetails::default()
        };
        assert_eq!(
            detail(State::CollectPin, &short, 0),
            "The PIN needs at least 6 characters."
        );
        assert!(
            detail(State::RequestFailed, &PinDetails::default(), 1).contains("isn't registered")
        );
        assert!(detail(State::RequestFailed, &PinDetails::default(), 99).contains("cancelled"));
    }
}
