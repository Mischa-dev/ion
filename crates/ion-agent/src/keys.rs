//! Model service API keys, kept with Ion's other secrets in
//! [`ion_keyring`] (the system keyring, or a private file when none
//! answers) under [`crate::provider::key_account`]. Never in config.

/// The secret name a service's key is saved under.
fn name(account: &str) -> String {
    format!("agent-key:{account}")
}

/// The key saved for `account`, if any. A keyring that can't be read
/// reads as no key, so the environment variable still works.
pub fn saved(account: &str) -> Option<String> {
    ion_keyring::global()
        .get(&name(account))
        .ok()
        .flatten()
        .filter(|k| !k.trim().is_empty())
}

/// Save `key` for `account`, replacing any saved before.
pub fn save(account: &str, key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("The key is empty.".to_owned());
    }
    ion_keyring::global()
        .set(&name(account), key)
        .map_err(|e| format!("Couldn't save the key: {e}"))
}

/// Forget the key saved for `account`. Nothing saved is not an error.
pub fn forget(account: &str) -> Result<(), String> {
    ion_keyring::global()
        .delete(&name(account))
        .map_err(|e| format!("Couldn't forget the key: {e}"))
}

/// Where keys are kept, for the setup to say: "system keyring",
/// "Keychain" or "a private file".
pub fn store_label() -> &'static str {
    ion_keyring::global().backend().label()
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
    lookup_in(account.and_then(saved), env)
}

fn lookup_in(saved: Option<String>, env: &str) -> (Option<String>, Source) {
    if let Some(key) = saved {
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
    fn saved_keys_then_environment_then_none() {
        assert_eq!(
            lookup_in(Some("sk-saved".into()), "PATH"),
            (Some("sk-saved".into()), Source::Keyring)
        );
        // PATH is always set; it stands in for a key variable.
        let (key, source) = lookup_in(None, "PATH");
        assert_eq!(source, Source::Environment);
        assert!(key.is_some());
        assert_eq!(
            lookup_in(None, "ION_TEST_KEY_THAT_IS_NOT_SET").1,
            Source::Missing
        );
        assert_eq!(lookup_in(None, " ").1, Source::NotNeeded);
        assert!(save("x", "  ").is_err());
    }
}
