//! User scripts Ion injects into pages.
//!
//! QtWebEngine reads Greasemonkey metadata (`@match`, `@include`, `@exclude`,
//! `@run-at`) from a script's source itself, so scripts are handed over with
//! their header intact and this module only needs `@name`, `@run-at` and
//! `@inject-into` to fill in the fields QML sets.
//!
//! Three kinds of script:
//! - per-site CSS from `[sites."host"] css = "…"`, turned into a script that
//!   adds a `<style>` element on matching pages;
//! - `*.user.js` files: Greasemonkey-style userscripts, run in an isolated
//!   world unless they ask for `@inject-into page`;
//! - `*.user.css` files: style sheets for every page, or only for the
//!   `@match` patterns listed in a leading `/* ==UserStyle== … */` block.

use std::path::Path;

use ion_config::Config;

/// When a script runs in the page's lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAt {
    /// Before any page script; the document is still empty.
    DocumentStart,
    /// When the DOM is ready (DOMContentLoaded).
    DocumentReady,
    /// After the page has loaded, or 500 ms after DOM ready.
    Deferred,
}

impl RunAt {
    /// The `WebEngineScript.InjectionPoint` name.
    pub fn as_str(self) -> &'static str {
        match self {
            RunAt::DocumentStart => "DocumentCreation",
            RunAt::DocumentReady => "DocumentReady",
            RunAt::Deferred => "Deferred",
        }
    }
}

/// Which JavaScript world a script runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum World {
    /// The page's own world: the script sees page globals, and the page can
    /// see and tamper with the script's.
    Main,
    /// An isolated world sharing only the DOM with the page.
    Isolated,
}

/// One script to install on the browser profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub name: String,
    pub source: String,
    pub run_at: RunAt,
    pub world: World,
}

/// A script that adds `css` to pages matching `matches` (every page when empty).
fn style_script(matches: &[String], css: &str) -> String {
    let mut out = String::from("// ==UserScript==\n");
    for m in matches {
        out.push_str(&format!("// @match {m}\n"));
    }
    out.push_str("// @run-at document-start\n// ==/UserScript==\n");
    // A JSON string is a valid JS string literal, whatever the CSS holds.
    let literal = serde_json::Value::String(css.to_owned()).to_string();
    // At document start there may be no element to attach to yet; wait for
    // the parser to create one.
    out.push_str(&format!(
        "(() => {{\n  const style = document.createElement('style');\n  \
         style.dataset.ion = 'user-style';\n  style.textContent = {literal};\n  \
         const add = () => {{\n    const root = document.head || document.documentElement;\n    \
         if (root) root.appendChild(style);\n    return !!root;\n  }};\n  \
         if (!add()) new MutationObserver((_, observer) => {{ if (add()) observer.disconnect(); }})\n    \
         .observe(document, {{ childList: true, subtree: true }});\n}})();\n"
    ));
    out
}

/// Metadata lines (`@key value`) of the header between `open` and `close`,
/// where each line may start with `//`, `*` or whitespace.
fn metadata<'a>(source: &'a str, open: &str, close: &str) -> Option<Vec<(&'a str, &'a str)>> {
    let mut lines = source.lines().map(str::trim);
    lines.by_ref().find(|l| l.contains(open))?;
    let mut out = Vec::new();
    for line in lines {
        if line.contains(close) {
            return Some(out);
        }
        let line = line.trim_start_matches(['/', '*', ' ', '\t']);
        if let Some(rest) = line.strip_prefix('@') {
            let (key, value) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
            out.push((key, value.trim()));
        }
    }
    None
}

/// A `.user.js` file as a script.
pub fn user_js(file_name: &str, source: &str) -> Script {
    let meta = metadata(source, "==UserScript==", "==/UserScript==").unwrap_or_default();
    let get = |key: &str| meta.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
    let run_at = match get("run-at") {
        Some("document-start") => RunAt::DocumentStart,
        Some("document-idle") => RunAt::Deferred,
        _ => RunAt::DocumentReady,
    };
    let world = match get("inject-into") {
        Some("page") => World::Main,
        _ => World::Isolated,
    };
    Script {
        name: get("name")
            .filter(|n| !n.is_empty())
            .unwrap_or(file_name)
            .to_owned(),
        source: source.to_owned(),
        run_at,
        world,
    }
}

/// A `.user.css` file as a script.
pub fn user_css(file_name: &str, source: &str) -> Script {
    let meta = metadata(source, "==UserStyle==", "==/UserStyle==").unwrap_or_default();
    let matches: Vec<String> = meta
        .iter()
        .filter(|(k, _)| *k == "match")
        .map(|(_, v)| (*v).to_owned())
        .collect();
    let name = meta
        .iter()
        .find(|(k, _)| *k == "name")
        .map_or(file_name, |(_, v)| *v);
    Script {
        name: name.to_owned(),
        source: style_script(&matches, source),
        run_at: RunAt::DocumentStart,
        world: World::Isolated,
    }
}

/// Every `*.user.js` and `*.user.css` file in `dir`, sorted by file name. A
/// missing directory has no scripts; unreadable files are reported in the
/// returned warnings and skipped.
pub fn load_dir(dir: &Path) -> (Vec<Script>, Vec<String>) {
    let mut scripts = Vec::new();
    let mut warnings = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (scripts, warnings);
    };
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let kind = if name.ends_with(".user.js") {
            user_js
        } else if name.ends_with(".user.css") {
            user_css
        } else {
            continue;
        };
        match std::fs::read_to_string(&path) {
            Ok(source) => scripts.push(kind(name, &source)),
            Err(e) => warnings.push(format!("{}: {e}", path.display())),
        }
    }
    (scripts, warnings)
}

/// Makes `navigator.globalPrivacyControl` true, matching the `Sec-GPC`
/// header. Runs in the page's world so page scripts see it.
pub fn global_privacy_control() -> Script {
    Script {
        name: "ion-global-privacy-control".to_owned(),
        source: "Object.defineProperty(Navigator.prototype, 'globalPrivacyControl', \
                 { get: () => true, configurable: true, enumerable: true });\n"
            .to_owned(),
        run_at: RunAt::DocumentStart,
        world: World::Main,
    }
}

/// Vim-style keys and link hints (`[keyboard] vim = true`).
pub fn keyboard_mode() -> Script {
    Script {
        name: "ion-keyboard-mode".to_owned(),
        source: include_str!("keyboard.js").to_owned(),
        run_at: RunAt::DocumentReady,
        world: World::Isolated,
    }
}

/// Ion's own scripts for `config` (privacy signals, per-site user agents,
/// keyboard mode) followed by the files in `dir`. `engine_user_agent` is the
/// engine's default user agent, which the `"chrome"` preset is made from.
pub fn all_scripts(
    config: &Config,
    engine_user_agent: &str,
    dir: Option<&Path>,
) -> (Vec<Script>, Vec<String>) {
    let mut scripts = Vec::new();
    scripts.extend(crate::agent::script(&config.sites, engine_user_agent));
    if config.privacy.global_privacy_control {
        scripts.push(global_privacy_control());
    }
    if config.keyboard.vim {
        scripts.push(keyboard_mode());
    }
    let mut warnings = Vec::new();
    if let Some(dir) = dir {
        let (files, problems) = load_dir(dir);
        scripts.extend(files);
        warnings.extend(problems);
    }
    (scripts, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_privacy_control_follows_config() {
        let mut config = Config::default();
        let names = |c: &Config| -> Vec<String> {
            all_scripts(c, "", None)
                .0
                .into_iter()
                .map(|s| s.name)
                .collect()
        };
        assert_eq!(names(&config), ["ion-global-privacy-control"]);
        config.privacy.global_privacy_control = false;
        assert!(names(&config).is_empty());
        config.keyboard.vim = true;
        assert_eq!(names(&config), ["ion-keyboard-mode"]);
    }

    #[test]
    fn userscript_metadata() {
        let src = "// ==UserScript==\n// @name   Tidy\n// @match https://x.org/*\n\
                   // @run-at document-start\n// @inject-into page\n// ==/UserScript==\nalert(1)";
        let s = user_js("tidy.user.js", src);
        assert_eq!(s.name, "Tidy");
        assert_eq!(s.run_at, RunAt::DocumentStart);
        assert_eq!(s.world, World::Main);
        assert_eq!(s.source, src);

        let plain = user_js("plain.user.js", "alert(1)");
        assert_eq!(plain.name, "plain.user.js");
        assert_eq!(plain.run_at, RunAt::DocumentReady);
        assert_eq!(plain.world, World::Isolated);
    }

    #[test]
    fn userstyle_matches_come_from_its_header() {
        let src = "/* ==UserStyle==\n * @name Wide\n * @match *://wiki.org/*\n==/UserStyle== */\nmain{max-width:none}";
        let s = user_css("wide.user.css", src);
        assert_eq!(s.name, "Wide");
        assert!(s.source.contains("// @match *://wiki.org/*\n"));
        assert!(s.source.contains("max-width:none"));
        let everywhere = user_css("x.user.css", "a{}");
        assert!(!everywhere.source.contains("@match"));
    }

    #[test]
    fn load_dir_reads_user_files_in_order() {
        let dir = std::env::temp_dir().join(format!("ion-sites-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("b.user.js"), "1").unwrap();
        std::fs::write(dir.join("a.user.css"), "a{}").unwrap();
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        let (scripts, warnings) = load_dir(&dir);
        let names: Vec<_> = scripts.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["a.user.css", "b.user.js"]);
        assert!(warnings.is_empty());
        assert!(load_dir(&dir.join("missing")).0.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
