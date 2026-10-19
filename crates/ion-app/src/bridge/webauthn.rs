//! `WebAuth` QML singleton: the words on the security key and passkey
//! dialog, from `ion_basics::webauthn`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[namespace = "ion"]
        type WebAuth = super::WebAuthRust;

        /// The heading for a `WebEngineWebAuthUxRequest` in `state` for
        /// `site` (its relying party id); `pin_reason` is its
        /// `pinRequest.reason`. Empty once the request is over.
        #[qinvokable]
        fn heading(self: &WebAuth, state: i32, site: &QString, pin_reason: i32) -> QString;

        /// The sentence under the heading. The PIN fields come from the
        /// request's `pinRequest`, `failure` is its `requestFailureReason`.
        #[qinvokable]
        fn detail(
            self: &WebAuth,
            state: i32,
            pin_reason: i32,
            pin_error: i32,
            min_pin_length: i32,
            attempts_left: i32,
            failure: i32,
        ) -> QString;
    }
}

use cxx_qt_lib::QString;
use ion_basics::webauthn::{self, PinDetails, PinError, PinReason, State};

#[derive(Default)]
pub struct WebAuthRust;

impl qobject::WebAuth {
    fn heading(&self, state: i32, site: &QString, pin_reason: i32) -> QString {
        let pin = PinDetails {
            reason: Some(PinReason::from_qt(pin_reason)),
            ..PinDetails::default()
        };
        QString::from(webauthn::heading(State::from_qt(state), &site.to_string(), &pin).as_str())
    }

    fn detail(
        &self,
        state: i32,
        pin_reason: i32,
        pin_error: i32,
        min_pin_length: i32,
        attempts_left: i32,
        failure: i32,
    ) -> QString {
        let pin = PinDetails {
            reason: Some(PinReason::from_qt(pin_reason)),
            error: Some(PinError::from_qt(pin_error)),
            min_length: min_pin_length,
            attempts_left,
        };
        QString::from(webauthn::detail(State::from_qt(state), &pin, failure).as_str())
    }
}
