//! Builds the cxx-qt bridges, the C++ shims and the `Ion` QML module.
//!
//! Bridges (`src/bridge/*.rs`), shims (`cpp/*.cpp`) and QML files (`qml/**/*.qml`)
//! are discovered automatically, so feature work can add files without editing
//! this script and colliding with other changes.

use std::path::{Path, PathBuf};

use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

fn collect(dir: &str, ext: &str) -> Vec<String> {
    fn walk(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, ext, out);
            } else if path.extension().is_some_and(|e| e == ext) {
                out.push(path);
            }
        }
    }
    println!("cargo::rerun-if-changed={dir}");
    let mut files = Vec::new();
    walk(Path::new(dir), ext, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect()
}

/// QML files whose first line is `pragma Singleton` are registered as singletons.
fn qml_file(path: String) -> QmlFile {
    let singleton = std::fs::read_to_string(&path)
        .is_ok_and(|src| src.trim_start().starts_with("pragma Singleton"));
    QmlFile::from(path).singleton(singleton)
}

fn main() {
    let bridges: Vec<String> = collect("src/bridge", "rs")
        .into_iter()
        .filter(|f| !f.ends_with("/mod.rs"))
        .collect();

    CxxQtBuilder::new_qml_module(
        QmlModule::new("Ion").qml_files(collect("qml", "qml").into_iter().map(qml_file)),
    )
    .qt_module("Gui")
    .qt_module("Network")
    .qt_module("Quick")
    .qt_module("QuickControls2")
    .qt_module("WebEngineCore")
    .qt_module("WebEngineQuick")
    .files(bridges)
    .cpp_files(collect("cpp", "cpp"))
    .build();
}
