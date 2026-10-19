//! Model service API keys, kept in the system keyring (Secret Service on
//! Linux, the Keychain on macOS) under [`crate::provider::key_account`].
//! Never in config, never in Ion's own files.

/// The keyring service name Ion's keys are filed under.
const SERVICE: &str = "Ion browser";

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn entry(account: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, account).map_err(describe)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn describe(error: keyring::Error) -> String {
    match error {
        keyring::Error::NoStorageAccess(_) | keyring::Error::PlatformFailure(_) => format!(
            "The system keyring isn't available ({error}). Start a Secret Service such as \
             GNOME Keyring or KeePassXC, or put the key in an environment variable."
        ),
        _ => format!("The system keyring refused: {error}"),
    }
}

/// The key saved for `account`, if any. A keyring that isn't there reads
/// as no key, so the environment variable still works.
pub fn saved(account: &str) -> Option<String> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        entry(account)
            .ok()?
            .get_password()
            .ok()
            .filter(|k| !k.trim().is_empty())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = account;
        None
    }
}

/// Save `key` for `account`, replacing any saved before.
pub fn save(account: &str, key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("The key is empty.".to_owned());
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        entry(account)?.set_password(key).map_err(describe)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = account;
        Err("Ion can't save keys on this system; use an environment variable.".to_owned())
    }
}

/// Forget the key saved for `account`. Nothing saved is not an error.
pub fn forget(account: &str) -> Result<(), String> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        match entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(describe(e)),
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = account;
        Ok(())
    }
}

/// Where a key comes from, for the setup to say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Keyring,
    /// The environment variable named in config.
    Environment,
    /// The service takes none.
    NotNeeded,
    Missing,
}

impl Source {
    pub fn id(self) -> &'static str {
        match self {
            Source::Keyring => "keyring",
            Source::Environment => "environment",
            Source::NotNeeded => "notNeeded",
            Source::Missing => "missing",
        }
    }
}

/// The key for a service at `account` (from [`crate::provider::key_account`])
/// whose config names `env`: the keyring first, then the environment. An
/// empty `env` means the service takes no key, unless one is saved.
pub fn lookup(account: Option<&str>, env: &str) -> (Option<String>, Source) {
    if let Some(key) = account.and_then(saved) {
        return (Some(key), Source::Keyring);
    }
    let env = env.trim();
    if env.is_empty() {
        return (None, Source::NotNeeded);
    }
    match std::env::var(env) {
        Ok(key) if !key.trim().is_empty() => (Some(key), Source::Environment),
        _ => (None, Source::Missing),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_and_keyless_services() {
        // No keyring in the test environment, and an account no one saved.
        let account = Some("openai:ion-test.invalid");
        assert_eq!(lookup(account, "").1, Source::NotNeeded);
        assert_eq!(
            lookup(account, "ION_TEST_KEY_THAT_IS_NOT_SET").1,
            Source::Missing
        );
        // PATH is always set; it stands in for a key variable.
        let (key, source) = lookup(None, "PATH");
        assert_eq!(source, Source::Environment);
        assert!(key.is_some());
        assert!(save("x", "  ").is_err());
    }
}
