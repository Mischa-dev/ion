//! Remembered rules on disk: `rules.json`, written atomically.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::rule::{Lifetime, Rule, Source};

const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct File {
    version: u32,
    rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
pub struct RuleStore {
    path: PathBuf,
}

impl RuleStore {
    pub fn new(path: impl Into<PathBuf>) -> RuleStore {
        RuleStore { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The saved rules. A missing file is no rules. Rules that are not the
    /// person's own forever rules are dropped, whatever the file says.
    pub fn load(&self) -> io::Result<Vec<Rule>> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let file: File = serde_json::from_str(&text).map_err(io::Error::other)?;
        if file.version > VERSION {
            return Err(io::Error::other(format!(
                "rules file version {} is newer than this Ion understands",
                file.version
            )));
        }
        Ok(file.rules.into_iter().filter(Self::persists).collect())
    }

    /// Save the rules that persist: the person's forever rules.
    pub fn save<'a>(&self, rules: impl IntoIterator<Item = &'a Rule>) -> io::Result<()> {
        let file = File {
            version: VERSION,
            rules: rules
                .into_iter()
                .filter(|r| Self::persists(r))
                .cloned()
                .collect(),
        };
        let json = serde_json::to_vec_pretty(&file).map_err(io::Error::other)?;
        write_atomic(&self.path, &json)
    }

    /// Move an unreadable file aside so it is not overwritten, and so the
    /// person can still recover it.
    pub fn set_aside(&self) -> io::Result<PathBuf> {
        let mut aside = self.path.as_os_str().to_owned();
        aside.push(".unreadable");
        let aside = PathBuf::from(aside);
        fs::rename(&self.path, &aside)?;
        Ok(aside)
    }

    fn persists(rule: &Rule) -> bool {
        rule.lifetime == Lifetime::Forever && rule.source == Source::User
    }
}

fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        crate::audit::create_private_dir(parent)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::test_dir;
    use crate::rule::{Effect, What, Who};
    use crate::site::SitePattern;

    fn rule(lifetime: Lifetime, source: Source) -> Rule {
        Rule {
            who: Who::Agent("ion".into()),
            what: What::Any,
            site: SitePattern::parse("github.com").unwrap(),
            effect: Effect::Allow,
            lifetime,
            source,
            created: 7,
        }
    }

    #[test]
    fn saves_only_forever_user_rules() {
        let dir = test_dir("store");
        let store = RuleStore::new(dir.join("safety/rules.json"));
        assert!(store.load().unwrap().is_empty());
        let keep = rule(Lifetime::Forever, Source::User);
        store
            .save(&[
                keep.clone(),
                rule(Lifetime::Session, Source::User),
                rule(Lifetime::Tab(1), Source::User),
                rule(Lifetime::Forever, Source::Config),
            ])
            .unwrap();
        assert_eq!(store.load().unwrap(), vec![keep]);
        assert!(!dir.join("safety/rules.json.tmp").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn hand_edited_session_rules_are_ignored() {
        let dir = test_dir("edited");
        let store = RuleStore::new(dir.join("rules.json"));
        let file = File {
            version: 1,
            rules: vec![
                rule(Lifetime::Session, Source::User),
                rule(Lifetime::Forever, Source::Config),
            ],
        };
        fs::create_dir_all(&dir).unwrap();
        fs::write(store.path(), serde_json::to_vec(&file).unwrap()).unwrap();
        assert!(store.load().unwrap().is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unreadable_files_error_and_can_be_set_aside() {
        let dir = test_dir("bad");
        fs::create_dir_all(&dir).unwrap();
        let store = RuleStore::new(dir.join("rules.json"));
        fs::write(store.path(), "{ nope").unwrap();
        assert!(store.load().is_err());
        let aside = store.set_aside().unwrap();
        assert!(aside.exists());
        assert!(store.load().unwrap().is_empty());

        fs::write(store.path(), r#"{"version":99,"rules":[]}"#).unwrap();
        assert!(store.load().is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
