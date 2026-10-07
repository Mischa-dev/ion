//! Import bookmarks and history from Chrome-family browsers and Firefox.
//!
//! [`find_profiles`] looks for each browser's default profile in its usual
//! place; [`import`] reads it. The browser may be running: its databases are
//! copied to a temporary directory first, so their locks don't matter.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use ion_session::HistoryEntry;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use crate::Bookmark;

/// History pages imported at most, most recently visited first. Matches the
/// cap of Ion's own history.
pub const MAX_HISTORY: usize = ion_session::history::MAX_ENTRIES;

/// Seconds between 1601-01-01 (Chrome's epoch) and 1970-01-01.
const WINDOWS_EPOCH_OFFSET: i64 = 11_644_473_600;

/// Which file layout a profile uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// Chrome, Chromium, Brave, Vivaldi, Edge: `Bookmarks` JSON + `History` SQLite.
    Chromium,
    /// Firefox: `places.sqlite` holds both.
    Firefox,
}

/// A browser profile found on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    /// Display name, e.g. "Firefox" or "Brave".
    pub browser: String,
    pub family: Family,
    pub dir: PathBuf,
}

/// What an import read.
#[derive(Debug, Default)]
pub struct Imported {
    pub bookmarks: Vec<Bookmark>,
    pub history: Vec<HistoryEntry>,
}

/// Default profiles of browsers installed for the user whose home is `home`.
pub fn find_profiles(home: &Path) -> Vec<Profile> {
    let mut found = Vec::new();
    let chromium: &[(&str, &str, &str)] = &[
        // (name, Linux dir under ~/.config, macOS dir under ~/Library/Application Support)
        ("Chrome", "google-chrome", "Google/Chrome"),
        ("Chromium", "chromium", "Chromium"),
        (
            "Brave",
            "BraveSoftware/Brave-Browser",
            "BraveSoftware/Brave-Browser",
        ),
        ("Vivaldi", "vivaldi", "Vivaldi"),
        ("Edge", "microsoft-edge", "Microsoft Edge"),
    ];
    for (name, linux, mac) in chromium {
        let dir = if cfg!(target_os = "macos") {
            home.join("Library/Application Support").join(mac)
        } else {
            home.join(".config").join(linux)
        }
        .join("Default");
        if dir.join("Bookmarks").is_file() || dir.join("History").is_file() {
            found.push(Profile {
                browser: (*name).to_owned(),
                family: Family::Chromium,
                dir,
            });
        }
    }
    let firefox_roots: Vec<PathBuf> = if cfg!(target_os = "macos") {
        vec![home.join("Library/Application Support/Firefox")]
    } else {
        vec![
            home.join(".mozilla/firefox"),
            home.join(".config/mozilla/firefox"),
            home.join("snap/firefox/common/.mozilla/firefox"),
        ]
    };
    for root in firefox_roots {
        if let Some(dir) = firefox_default_profile(&root) {
            found.push(Profile {
                browser: "Firefox".to_owned(),
                family: Family::Firefox,
                dir,
            });
            break;
        }
    }
    found
}

/// The default profile listed in `<root>/profiles.ini`.
///
/// Prefers the `[Install…]` section's `Default` (the profile Firefox actually
/// opens), then a `[Profile…]` with `Default=1`, then the first profile.
fn firefox_default_profile(root: &Path) -> Option<PathBuf> {
    let ini = std::fs::read_to_string(root.join("profiles.ini")).ok()?;
    let mut install_default = None;
    let mut profiles: Vec<(String, bool, bool)> = Vec::new(); // (path, relative, default)
    let mut section = String::new();
    for line in ini.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.to_owned();
            if section.starts_with("Profile") {
                profiles.push((String::new(), true, false));
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if section.starts_with("Install") && key == "Default" {
            install_default.get_or_insert_with(|| value.to_owned());
        } else if section.starts_with("Profile") {
            if let Some(p) = profiles.last_mut() {
                match key {
                    "Path" => p.0 = value.to_owned(),
                    "IsRelative" => p.1 = value != "0",
                    "Default" => p.2 = value == "1",
                    _ => {}
                }
            }
        }
    }
    let resolve = |path: &str, relative: bool| {
        if relative {
            root.join(path)
        } else {
            PathBuf::from(path)
        }
    };
    let candidate = install_default.map(|p| resolve(&p, true)).or_else(|| {
        profiles
            .iter()
            .find(|p| p.2)
            .or(profiles.first())
            .map(|p| resolve(&p.0, p.1))
    })?;
    candidate
        .join("places.sqlite")
        .is_file()
        .then_some(candidate)
}

/// Read a profile's bookmarks and history. Bookmarks go in a folder named
/// "Imported from <browser>".
pub fn import(profile: &Profile) -> io::Result<Imported> {
    let folder = format!("Imported from {}", profile.browser);
    match profile.family {
        Family::Chromium => {
            let bookmarks = match std::fs::read_to_string(profile.dir.join("Bookmarks")) {
                Ok(json) => chrome_bookmarks(&json, &folder)?,
                Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
                Err(e) => return Err(e),
            };
            let history_db = profile.dir.join("History");
            let history = if history_db.is_file() {
                with_copy(&history_db, chrome_history)?
            } else {
                Vec::new()
            };
            Ok(Imported { bookmarks, history })
        }
        Family::Firefox => with_copy(&profile.dir.join("places.sqlite"), |conn| {
            Ok(Imported {
                bookmarks: firefox_bookmarks(conn, &folder)?,
                history: firefox_history(conn)?,
            })
        }),
    }
}

/// Bookmarks from a Chrome `Bookmarks` file, under `folder`.
pub fn chrome_bookmarks(json: &str, folder: &str) -> io::Result<Vec<Bookmark>> {
    let root: Value =
        serde_json::from_str(json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut out = Vec::new();
    if let Some(roots) = root.get("roots").and_then(Value::as_object) {
        for key in ["bookmark_bar", "other", "synced"] {
            if let Some(node) = roots.get(key) {
                walk_chrome(node, folder, &mut out);
            }
        }
    }
    Ok(out)
}

fn walk_chrome(node: &Value, parent: &str, out: &mut Vec<Bookmark>) {
    let name = node.get("name").and_then(Value::as_str).unwrap_or("");
    match node.get("type").and_then(Value::as_str) {
        Some("url") => {
            let Some(url) = node.get("url").and_then(Value::as_str) else {
                return;
            };
            let added = node
                .get("date_added")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<i64>().ok())
                .map_or(0, chrome_time);
            out.push(Bookmark {
                url: url.to_owned(),
                title: name.to_owned(),
                folder: parent.to_owned(),
                added,
            });
        }
        Some("folder") => {
            let path = join_folder(parent, name);
            for child in node
                .get("children")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                walk_chrome(child, &path, out);
            }
        }
        _ => {}
    }
}

fn join_folder(parent: &str, name: &str) -> String {
    match (parent.is_empty(), name.is_empty()) {
        (_, true) => parent.to_owned(),
        (true, false) => name.to_owned(),
        (false, false) => format!("{parent}/{name}"),
    }
}

/// Chrome timestamps are microseconds since 1601; Ion uses Unix seconds.
fn chrome_time(micros: i64) -> u64 {
    u64::try_from(micros / 1_000_000 - WINDOWS_EPOCH_OFFSET).unwrap_or(0)
}

/// Firefox timestamps are microseconds since 1970.
fn firefox_time(micros: i64) -> u64 {
    u64::try_from(micros / 1_000_000).unwrap_or(0)
}

/// History from a Chrome `History` database.
pub fn chrome_history(conn: &Connection) -> io::Result<Vec<HistoryEntry>> {
    query_history(
        conn,
        "SELECT url, title, visit_count, last_visit_time FROM urls
         WHERE hidden = 0 AND visit_count > 0
         ORDER BY last_visit_time DESC LIMIT ?1",
        chrome_time,
    )
}

/// History from a Firefox `places.sqlite` database.
pub fn firefox_history(conn: &Connection) -> io::Result<Vec<HistoryEntry>> {
    query_history(
        conn,
        "SELECT url, title, visit_count, last_visit_date FROM moz_places
         WHERE hidden = 0 AND visit_count > 0 AND last_visit_date IS NOT NULL
         ORDER BY last_visit_date DESC LIMIT ?1",
        firefox_time,
    )
}

fn query_history(
    conn: &Connection,
    sql: &str,
    time: fn(i64) -> u64,
) -> io::Result<Vec<HistoryEntry>> {
    let mut stmt = conn.prepare(sql).map_err(sql_err)?;
    let limit = i64::try_from(MAX_HISTORY).unwrap_or(i64::MAX);
    let rows = stmt
        .query_map([limit], |row| {
            Ok(HistoryEntry {
                url: row.get(0)?,
                title: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                visits: u32::try_from(row.get::<_, i64>(2)?).unwrap_or(1).max(1),
                last_visit: time(row.get::<_, Option<i64>>(3)?.unwrap_or(0)),
            })
        })
        .map_err(sql_err)?;
    rows.filter(|r| {
        r.as_ref()
            .map_or(true, |e| ion_session::history::is_recordable(&e.url))
    })
    .collect::<Result<_, _>>()
    .map_err(sql_err)
}

/// Bookmarks from a Firefox `places.sqlite` database, under `folder`.
/// Tags are skipped; they are stored as folders but aren't folders.
pub fn firefox_bookmarks(conn: &Connection, folder: &str) -> io::Result<Vec<Bookmark>> {
    // Every folder, so each bookmark's path can be built from its parents.
    let mut folders: HashMap<i64, (i64, String, String)> = HashMap::new(); // id -> (parent, title, guid)
    {
        let mut stmt = conn
            .prepare("SELECT id, parent, title, guid FROM moz_bookmarks WHERE type = 2")
            .map_err(sql_err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (id, parent, title, guid) = row.map_err(sql_err)?;
            folders.insert(id, (parent, title, guid));
        }
    }
    // A root's display name, or None for the tags root.
    let root_name = |guid: &str, title: &str| -> Option<String> {
        Some(
            match guid {
                "root________" => "",
                "toolbar_____" => "Toolbar",
                "menu________" => "Menu",
                "unfiled_____" => "Other",
                "mobile______" => "Mobile",
                "tags________" => return None,
                _ => title,
            }
            .to_owned(),
        )
    };
    let path_of = |mut id: i64| -> Option<String> {
        let mut parts = Vec::new();
        // Bounded walk: a corrupt database with a cycle can't hang the import.
        for _ in 0..64 {
            let Some((parent, title, guid)) = folders.get(&id) else {
                break;
            };
            parts.push(root_name(guid, title)?);
            if guid == "root________" {
                break;
            }
            id = *parent;
        }
        let path = parts
            .iter()
            .rev()
            .filter(|p| !p.is_empty())
            .fold(folder.to_owned(), |acc, p| join_folder(&acc, p));
        Some(path)
    };

    let mut stmt = conn
        .prepare(
            "SELECT p.url, b.title, b.parent, b.dateAdded FROM moz_bookmarks b
             JOIN moz_places p ON p.id = b.fk
             WHERE b.type = 1 ORDER BY b.parent, b.position",
        )
        .map_err(sql_err)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                row.get::<_, i64>(2)?,
                row.get::<_, Option<i64>>(3)?.unwrap_or(0),
            ))
        })
        .map_err(sql_err)?;
    let mut out = Vec::new();
    for row in rows {
        let (url, title, parent, added) = row.map_err(sql_err)?;
        if let Some(folder) = path_of(parent) {
            out.push(Bookmark {
                url,
                title,
                folder,
                added: firefox_time(added),
            });
        }
    }
    Ok(out)
}

/// Open a copy of the SQLite database at `db` (with its `-wal` file, so
/// recent changes a running browser hasn't checkpointed are included) and run
/// `read` on it.
fn with_copy<T>(db: &Path, read: impl FnOnce(&Connection) -> io::Result<T>) -> io::Result<T> {
    let tmp = std::env::temp_dir().join(format!(
        "ion-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir_all(&tmp)?;
    let result = (|| {
        let copy = tmp.join("db.sqlite");
        std::fs::copy(db, &copy)?;
        let mut wal = db.as_os_str().to_owned();
        wal.push("-wal");
        if Path::new(&wal).is_file() {
            std::fs::copy(&wal, tmp.join("db.sqlite-wal"))?;
        }
        let conn = Connection::open_with_flags(
            &copy,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(sql_err)?;
        read(&conn)
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

fn sql_err(e: rusqlite::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ion-import-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const CHROME_BOOKMARKS: &str = r#"{
      "roots": {
        "bookmark_bar": {
          "type": "folder", "name": "Bookmarks bar",
          "children": [
            { "type": "url", "name": "Nix", "url": "https://nixos.org/",
              "date_added": "13370000000000000" },
            { "type": "folder", "name": "Rust", "children": [
              { "type": "url", "name": "Docs", "url": "https://docs.rs/" }
            ]}
          ]
        },
        "other": { "type": "folder", "name": "Other bookmarks", "children": [
          { "type": "url", "name": "Settings", "url": "chrome://settings" }
        ]},
        "synced": { "type": "folder", "name": "Mobile bookmarks", "children": [] }
      }
    }"#;

    #[test]
    fn chrome_bookmarks_keep_folders_and_dates() {
        let b = chrome_bookmarks(CHROME_BOOKMARKS, "Imported from Chrome").unwrap();
        assert_eq!(b.len(), 3);
        assert_eq!(b[0].url, "https://nixos.org/");
        assert_eq!(b[0].folder, "Imported from Chrome/Bookmarks bar");
        assert_eq!(b[0].added, 13_370_000_000 - 11_644_473_600);
        assert_eq!(b[1].folder, "Imported from Chrome/Bookmarks bar/Rust");
        // Internal pages are read here and dropped by `Bookmarks::extend`.
        assert_eq!(b[2].url, "chrome://settings");
    }

    #[test]
    fn chrome_bookmarks_reject_garbage() {
        assert!(chrome_bookmarks("not json", "x").is_err());
        assert!(chrome_bookmarks("{}", "x").unwrap().is_empty());
    }

    fn chrome_history_db(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE urls (id INTEGER PRIMARY KEY, url TEXT, title TEXT,
               visit_count INTEGER, typed_count INTEGER, last_visit_time INTEGER, hidden INTEGER);
             INSERT INTO urls VALUES (1, 'https://a.example/', 'A', 3, 0, 13370000000000000, 0);
             INSERT INTO urls VALUES (2, 'https://b.example/', NULL, 1, 0, 13380000000000000, 0);
             INSERT INTO urls VALUES (3, 'https://hidden.example/', 'H', 1, 0, 13390000000000000, 1);
             INSERT INTO urls VALUES (4, 'chrome://newtab/', 'New tab', 9, 0, 13390000000000000, 0);",
        )
        .unwrap();
    }

    #[test]
    fn chrome_profile_import() {
        let dir = temp("chrome");
        std::fs::write(dir.join("Bookmarks"), CHROME_BOOKMARKS).unwrap();
        chrome_history_db(&dir.join("History"));
        let imported = import(&Profile {
            browser: "Chrome".into(),
            family: Family::Chromium,
            dir: dir.clone(),
        })
        .unwrap();
        assert_eq!(imported.bookmarks.len(), 3);
        let urls: Vec<_> = imported.history.iter().map(|e| e.url.as_str()).collect();
        assert_eq!(urls, ["https://b.example/", "https://a.example/"]);
        assert_eq!(imported.history[1].visits, 3);
        assert_eq!(
            imported.history[1].last_visit,
            13_370_000_000 - 11_644_473_600
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn places_db(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE moz_places (id INTEGER PRIMARY KEY, url TEXT, title TEXT,
               visit_count INTEGER, hidden INTEGER, last_visit_date INTEGER);
             CREATE TABLE moz_bookmarks (id INTEGER PRIMARY KEY, type INTEGER, fk INTEGER,
               parent INTEGER, position INTEGER, title TEXT, dateAdded INTEGER, guid TEXT);
             INSERT INTO moz_places VALUES (1, 'https://nixos.org/', 'NixOS', 5, 0, 1800000000000000);
             INSERT INTO moz_places VALUES (2, 'https://docs.rs/', 'Docs', 0, 0, NULL);
             INSERT INTO moz_places VALUES (3, 'https://tagged.example/', 'T', 1, 0, 1700000000000000);
             INSERT INTO moz_bookmarks VALUES (1, 2, NULL, 0, 0, '', 0, 'root________');
             INSERT INTO moz_bookmarks VALUES (2, 2, NULL, 1, 0, 'toolbar', 0, 'toolbar_____');
             INSERT INTO moz_bookmarks VALUES (3, 2, NULL, 1, 1, 'tags', 0, 'tags________');
             INSERT INTO moz_bookmarks VALUES (4, 2, NULL, 2, 0, 'Dev', 0, 'devfolder___');
             INSERT INTO moz_bookmarks VALUES (5, 1, 1, 2, 1, 'NixOS', 1600000000000000, 'b1');
             INSERT INTO moz_bookmarks VALUES (6, 1, 2, 4, 0, 'Docs', 0, 'b2');
             INSERT INTO moz_bookmarks VALUES (7, 2, NULL, 3, 0, 'mytag', 0, 'tag1');
             INSERT INTO moz_bookmarks VALUES (8, 1, 3, 7, 0, NULL, 0, 'b3');",
        )
        .unwrap();
    }

    #[test]
    fn firefox_profile_import() {
        let dir = temp("firefox");
        places_db(&dir.join("places.sqlite"));
        let imported = import(&Profile {
            browser: "Firefox".into(),
            family: Family::Firefox,
            dir: dir.clone(),
        })
        .unwrap();
        let b: Vec<_> = imported
            .bookmarks
            .iter()
            .map(|b| (b.url.as_str(), b.folder.as_str()))
            .collect();
        assert_eq!(
            b,
            [
                ("https://nixos.org/", "Imported from Firefox/Toolbar"),
                ("https://docs.rs/", "Imported from Firefox/Toolbar/Dev"),
            ]
        );
        assert_eq!(imported.bookmarks[0].added, 1_600_000_000);
        let h: Vec<_> = imported.history.iter().map(|e| e.url.as_str()).collect();
        assert_eq!(h, ["https://nixos.org/", "https://tagged.example/"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finds_firefox_default_profile_from_install_section() {
        let home = temp("home");
        let root = if cfg!(target_os = "macos") {
            home.join("Library/Application Support/Firefox")
        } else {
            home.join(".mozilla/firefox")
        };
        std::fs::create_dir_all(root.join("Profiles/abc.default-release")).unwrap();
        std::fs::create_dir_all(root.join("Profiles/old.default")).unwrap();
        places_db(&root.join("Profiles/abc.default-release/places.sqlite"));
        places_db(&root.join("Profiles/old.default/places.sqlite"));
        std::fs::write(
            root.join("profiles.ini"),
            "[Profile0]\nName=old\nIsRelative=1\nPath=Profiles/old.default\nDefault=1\n\n\
             [Install4F96D1932A9F858E]\nDefault=Profiles/abc.default-release\nLocked=1\n",
        )
        .unwrap();
        let found = find_profiles(&home);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].family, Family::Firefox);
        assert!(found[0].dir.ends_with("Profiles/abc.default-release"));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn finds_chromium_profiles() {
        let home = temp("home-chrome");
        let dir = if cfg!(target_os = "macos") {
            home.join("Library/Application Support/BraveSoftware/Brave-Browser/Default")
        } else {
            home.join(".config/BraveSoftware/Brave-Browser/Default")
        };
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Bookmarks"), CHROME_BOOKMARKS).unwrap();
        let found = find_profiles(&home);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].browser, "Brave");
        let _ = std::fs::remove_dir_all(&home);
    }
}
