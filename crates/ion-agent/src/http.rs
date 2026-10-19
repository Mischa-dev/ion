//! What the model services share: one HTTP client setup, and sending a
//! request again when the service is busy or the network hiccups.

use std::time::Duration;

use serde_json::Value;

/// Answers bigger than this are not a chat reply.
pub(crate) const MAX_REPLY_BYTES: u64 = 4 * 1024 * 1024;

/// Tries per request: the first, then two more while the service is busy.
const ATTEMPTS: u32 = 3;

/// The longest wait between tries, even when the service asks for more.
const MAX_WAIT: Duration = Duration::from_secs(10);

/// A client for model services. Honours the usual `HTTPS_PROXY` variables.
pub(crate) fn client() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(180)))
        // Error bodies say what went wrong; read them instead of
        // getting a bare status.
        .http_status_as_error(false)
        .user_agent(concat!("Ion/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// Whether a response with `status` is worth asking again for: rate
/// limits, overload and the server's own failures.
pub(crate) fn is_retryable(status: u16) -> bool {
    matches!(status, 408 | 409 | 429 | 500 | 502 | 503 | 504 | 529)
}

/// How long to wait before try `attempt` (1 for the second try): the
/// service's `retry-after` when it gives one, otherwise a growing pause.
pub(crate) fn backoff(attempt: u32, retry_after: Option<&str>) -> Duration {
    let asked = retry_after
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s >= 0.0)
        .map(Duration::from_secs_f64);
    asked
        .unwrap_or_else(|| Duration::from_millis(800 * 3u64.pow(attempt.saturating_sub(1))))
        .min(MAX_WAIT)
}

/// POST `body` as JSON to `url` with `headers`, trying again while the
/// service is busy. The response is returned whatever its status once the
/// tries run out, so callers can read the service's own error message.
pub(crate) fn post_json(
    http: &ureq::Agent,
    url: &str,
    headers: &[(&str, String)],
    body: &Value,
) -> Result<ureq::http::Response<ureq::Body>, String> {
    let body = body.to_string();
    let mut attempt = 0;
    loop {
        let mut request = http.post(url).header("Content-Type", "application/json");
        for (name, value) in headers {
            request = request.header(*name, value);
        }
        attempt += 1;
        let last = attempt >= ATTEMPTS;
        match request.send(body.as_str()) {
            Ok(response) if !last && is_retryable(response.status().as_u16()) => {
                let after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok());
                std::thread::sleep(backoff(attempt, after));
            }
            Ok(response) => return Ok(response),
            Err(e) if !last && is_transient(&e) => std::thread::sleep(backoff(attempt, None)),
            Err(e) => return Err(format!("Couldn't reach the model service: {e}")),
        }
    }
}

/// Network failures that may go away on their own.
fn is_transient(error: &ureq::Error) -> bool {
    matches!(
        error,
        ureq::Error::Io(_) | ureq::Error::Timeout(_) | ureq::Error::ConnectionFailed
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busy_services_are_asked_again() {
        assert!(is_retryable(429));
        assert!(is_retryable(529));
        assert!(is_retryable(503));
        assert!(!is_retryable(400));
        assert!(!is_retryable(401));
        assert!(!is_retryable(404));
    }

    #[test]
    fn waits_grow_and_follow_the_service() {
        assert_eq!(backoff(1, None), Duration::from_millis(800));
        assert_eq!(backoff(2, None), Duration::from_millis(2400));
        assert_eq!(backoff(1, Some("2")), Duration::from_secs(2));
        assert_eq!(backoff(1, Some("0.5")), Duration::from_millis(500));
        assert_eq!(backoff(1, Some("3600")), MAX_WAIT);
        assert_eq!(backoff(1, Some("soon")), Duration::from_millis(800));
    }
}
