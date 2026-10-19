//! Ion's privacy protections, without Qt.
//!
//! - [`https`]: HTTPS-first navigation with session fallbacks.
//! - [`fingerprint_script`]: the page script that resists fingerprinting.
//! - [`window`]: private windows and profiles.
//!
//! Tracker blocking itself is `ion-adblock`; third-party cookies and Global
//! Privacy Control are applied by the app's `Privacy` bridge.

pub mod https;
pub mod window;

pub use https::{Https, HttpsMode, Upgrade};

/// The fingerprinting-protection script for pages, to run in the page's
/// world at document creation in every frame. `seed` should be random per
/// session; sites in `off` (and their subdomains) are left alone.
pub fn fingerprint_script<'a>(seed: u32, off: impl IntoIterator<Item = &'a str>) -> String {
    let off: Vec<&str> = off.into_iter().collect();
    include_str!("fingerprint.js")
        .replace("__ION_SEED__", &seed.to_string())
        .replace(
            "__ION_OFF_SITES__",
            &serde_json::to_string(&off).unwrap_or_else(|_| "[]".into()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_carries_its_seed_and_exceptions() {
        let script = fingerprint_script(42, ["a.com", "quote\"d"]);
        assert!(script.contains("const SEED = 42;"));
        assert!(script.contains(r#"const OFF = ["a.com","quote\"d"];"#));
        assert!(!script.contains("__ION_"));
    }
}
