//! Which Chrome extensions Ion loads at startup.
//!
//! Two sources, both unpacked extension folders (a folder with a
//! `manifest.json`): the `extensions` list in config (what the Nix module
//! writes, often store paths), and every folder inside Ion's own
//! `extensions` folder in the data directory. QtWebEngine runs Manifest V3
//! only, so older extensions are reported instead of loaded.

use std::path::{Path, PathBuf};

/// An extension folder to load, or why one can't be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    Load(PathBuf),
    Skip { path: PathBuf, reason: String },
}

/// Check one folder.
pub fn inspect(path: &Path) -> Found {
    let skip = |reason: String| Found::Skip {
        path: path.to_owned(),
        reason,
    };
    let manifest = match std::fs::read_to_string(path.join("manifest.json")) {
        Ok(text) => text,
        Err(e) => return skip(format!("no readable manifest.json ({e})")),
    };
    let manifest: serde_json::Value = match serde_json::from_str(&manifest) {
        Ok(v) => v,
        Err(e) => return skip(format!("manifest.json is not valid JSON ({e})")),
    };
    match manifest["manifest_version"].as_u64() {
        Some(3) => Found::Load(path.to_owned()),
        Some(v) => skip(format!(
            "Manifest V{v}; Ion's engine only runs Manifest V3 extensions"
        )),
        None => skip("manifest.json has no manifest_version".to_owned()),
    }
}

/// Every extension to consider: the configured paths (with a leading `~/`
/// expanded against `home`), then the sub-folders of `folder` in name order.
/// Duplicates are dropped.
pub fn discover(configured: &[String], folder: Option<&Path>, home: Option<&Path>) -> Vec<Found> {
    let mut paths: Vec<PathBuf> = configured
        .iter()
        .map(|p| match (p.strip_prefix("~/"), home) {
            (Some(rest), Some(home)) => home.join(rest),
            _ => PathBuf::from(p),
        })
        .collect();
    if let Some(folder) = folder {
        if let Ok(entries) = std::fs::read_dir(folder) {
            let mut dirs: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir())
                .collect();
            dirs.sort();
            paths.extend(dirs);
        }
    }
    let mut seen = std::collections::HashSet::new();
    paths
        .into_iter()
        .filter(|p| seen.insert(p.clone()))
        .map(|p| inspect(&p))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ion-ext-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn extension(dir: &Path, version: u32) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("manifest.json"),
            format!(r#"{{"name":"x","version":"1","manifest_version":{version}}}"#),
        )
        .unwrap();
    }

    #[test]
    fn loads_mv3_and_explains_the_rest() {
        let root = temp("discover");
        let folder = root.join("extensions");
        extension(&folder.join("b-mv3"), 3);
        extension(&folder.join("a-mv2"), 2);
        std::fs::create_dir_all(folder.join("c-empty")).unwrap();
        std::fs::write(folder.join("notes.txt"), "").unwrap();
        let home = root.join("home");
        extension(&home.join("dev/ext"), 3);

        let found = discover(
            &[
                "~/dev/ext".to_owned(),
                folder.join("b-mv3").to_string_lossy().into(),
            ],
            Some(&folder),
            Some(&home),
        );
        assert_eq!(found.len(), 4, "{found:?}");
        assert_eq!(found[0], Found::Load(home.join("dev/ext")));
        assert_eq!(found[1], Found::Load(folder.join("b-mv3")));
        assert!(matches!(&found[2], Found::Skip { reason, .. } if reason.contains("Manifest V2")));
        assert!(
            matches!(&found[3], Found::Skip { reason, .. } if reason.contains("manifest.json"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_folder_is_fine() {
        assert!(discover(&[], Some(Path::new("/nonexistent/ion")), None).is_empty());
    }
}
