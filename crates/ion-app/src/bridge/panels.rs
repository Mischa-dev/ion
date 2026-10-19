//! `Panels` QML singleton: the side panel's web panels and notes
//! (`ion_panels`), saved as `panels.json` and `notes.json` in Ion's data
//! directory after every change.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(i32, revision, READ, NOTIFY)]
        #[qproperty(i32, notes_revision, cxx_name = "notesRevision", READ, NOTIFY)]
        #[qproperty(QString, open, READ, NOTIFY)]
        #[qproperty(i32, width, READ, NOTIFY)]
        #[namespace = "ion"]
        type Panels = super::PanelsRust;

        /// The web panels in order, as `{ id, url, label }`.
        #[qinvokable]
        #[cxx_name = "webPanels"]
        fn web_panels(self: &Panels) -> QVariant;

        /// Add `url` as a web panel (or find it); returns its id, or "" if
        /// it isn't a web page.
        #[qinvokable]
        fn add(self: Pin<&mut Panels>, url: &QString, title: &QString) -> QString;

        #[qinvokable]
        fn remove(self: Pin<&mut Panels>, id: &QString);

        #[qinvokable]
        #[cxx_name = "moveTo"]
        fn move_to(self: Pin<&mut Panels>, id: &QString, index: i32);

        #[qinvokable]
        #[cxx_name = "setWidth"]
        fn set_width(self: Pin<&mut Panels>, width: i32);

        /// Show "notes" or a web panel's id; "" closes the panel.
        #[qinvokable]
        fn show(self: Pin<&mut Panels>, what: &QString);

        /// Show `what`, or close it if it is showing.
        #[qinvokable]
        fn toggle(self: Pin<&mut Panels>, what: &QString);

        /// Notes matching `query`, newest first, as `{ id, title, text, url,
        /// pageTitle, updated }` (`updated` in seconds since the epoch).
        #[qinvokable]
        fn notes(self: &Panels, query: &QString) -> QVariant;

        /// A new note; returns its id.
        #[qinvokable]
        #[cxx_name = "addNote"]
        fn add_note(self: Pin<&mut Panels>, text: &QString) -> i32;

        /// Quote `selection` from the page in a note; returns the note's id.
        #[qinvokable]
        fn clip(self: Pin<&mut Panels>, selection: &QString, url: &QString, title: &QString)
        -> i32;

        /// Replace a note's text; an empty note is deleted.
        #[qinvokable]
        #[cxx_name = "setNoteText"]
        fn set_note_text(self: Pin<&mut Panels>, id: i32, text: &QString);

        #[qinvokable]
        #[cxx_name = "removeNote"]
        fn remove_note(self: Pin<&mut Panels>, id: i32);

        #[qinvokable]
        #[cxx_name = "noteText"]
        fn note_text(self: &Panels, id: i32) -> QString;
    }

    impl cxx_qt::Initialize for Panels {}
}

use std::path::PathBuf;
use std::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QVariant};
use ion_panels::{Notes, Panels};

#[derive(Default)]
pub struct PanelsRust {
    revision: i32,
    notes_revision: i32,
    open: QString,
    width: i32,
    panels: Panels,
    notes: Notes,
}

fn file(name: &str) -> Option<PathBuf> {
    Some(ion_session::paths::data_dir()?.join(name))
}

fn read(name: &str) -> String {
    file(name)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default()
}

/// Write via a temporary file, so a crash never leaves half a file.
fn write(name: &str, text: &str) {
    let Some(path) = file(name) else {
        return;
    };
    let tmp = path.with_extension("json.tmp");
    let result = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&tmp, text))
        .and_then(|()| std::fs::rename(&tmp, &path));
    if let Err(e) = result {
        eprintln!("ion: {}: {e}", path.display());
    }
}

fn now() -> u64 {
    ion_session::history::now()
}

fn map(entries: Vec<(&str, QVariant)>) -> QVariant {
    let mut row = QMap::<QMapPair_QString_QVariant>::default();
    for (key, value) in entries {
        row.insert(QString::from(key), value);
    }
    QVariant::from(&row)
}

fn text(value: &str) -> QVariant {
    QVariant::from(&QString::from(value))
}

impl cxx_qt::Initialize for qobject::Panels {
    fn initialize(mut self: Pin<&mut Self>) {
        let panels = Panels::from_json(&read("panels.json"));
        let notes = Notes::from_json(&read("notes.json"));
        let mut rust = self.as_mut().rust_mut();
        rust.panels = panels;
        rust.notes = notes;
        self.panels_changed(false);
    }
}

impl qobject::Panels {
    /// Publish the panel state and, if `save`, write it.
    fn panels_changed(mut self: Pin<&mut Self>, save: bool) {
        if save {
            write("panels.json", &self.panels.to_json());
        }
        let open = QString::from(self.panels.open.as_str());
        if self.open != open {
            self.as_mut().rust_mut().open = open;
            self.as_mut().open_changed();
        }
        let width = i32::try_from(self.panels.width).unwrap_or(i32::MAX);
        if self.width != width {
            self.as_mut().rust_mut().width = width;
            self.as_mut().width_changed();
        }
        let revision = self.revision.wrapping_add(1);
        self.as_mut().rust_mut().revision = revision;
        self.revision_changed();
    }

    fn notes_changed(mut self: Pin<&mut Self>) {
        write("notes.json", &self.notes.to_json());
        let revision = self.notes_revision.wrapping_add(1);
        self.as_mut().rust_mut().notes_revision = revision;
        self.notes_revision_changed();
    }

    fn web_panels(&self) -> QVariant {
        let mut rows = QList::<QVariant>::default();
        for panel in &self.panels.panels {
            rows.append(map(vec![
                ("id", text(&panel.id)),
                ("url", text(&panel.url)),
                ("label", text(&panel.label())),
            ]));
        }
        QVariant::from(&rows)
    }

    fn add(mut self: Pin<&mut Self>, url: &QString, title: &QString) -> QString {
        let id = self
            .as_mut()
            .rust_mut()
            .panels
            .add(&url.to_string(), &title.to_string());
        if id.is_some() {
            self.as_mut().panels_changed(true);
        }
        QString::from(id.unwrap_or_default().as_str())
    }

    fn remove(mut self: Pin<&mut Self>, id: &QString) {
        if self.as_mut().rust_mut().panels.remove(&id.to_string()) {
            self.panels_changed(true);
        }
    }

    fn move_to(mut self: Pin<&mut Self>, id: &QString, index: i32) {
        let index = usize::try_from(index).unwrap_or(0);
        if self
            .as_mut()
            .rust_mut()
            .panels
            .move_to(&id.to_string(), index)
        {
            self.panels_changed(true);
        }
    }

    fn set_width(mut self: Pin<&mut Self>, width: i32) {
        let width = u32::try_from(width).unwrap_or(0);
        self.as_mut().rust_mut().panels.set_width(width);
        self.panels_changed(true);
    }

    fn show(mut self: Pin<&mut Self>, what: &QString) {
        self.as_mut().rust_mut().panels.show(&what.to_string());
        self.panels_changed(true);
    }

    fn toggle(mut self: Pin<&mut Self>, what: &QString) {
        self.as_mut().rust_mut().panels.toggle(&what.to_string());
        self.panels_changed(true);
    }

    fn notes(&self, query: &QString) -> QVariant {
        let mut rows = QList::<QVariant>::default();
        for note in self.notes.search(&query.to_string()) {
            rows.append(map(vec![
                ("id", QVariant::from(&i32::try_from(note.id).unwrap_or(-1))),
                ("title", text(note.title())),
                ("text", text(&note.text)),
                ("url", text(&note.url)),
                ("pageTitle", text(&note.page_title)),
                (
                    "updated",
                    QVariant::from(&f64::from(u32::try_from(note.updated).unwrap_or(u32::MAX))),
                ),
            ]));
        }
        QVariant::from(&rows)
    }

    fn add_note(mut self: Pin<&mut Self>, text: &QString) -> i32 {
        let id = self.as_mut().rust_mut().notes.add(&text.to_string(), now());
        self.notes_changed();
        i32::try_from(id).unwrap_or(-1)
    }

    fn clip(mut self: Pin<&mut Self>, selection: &QString, url: &QString, title: &QString) -> i32 {
        if selection.to_string().trim().is_empty() {
            return -1;
        }
        let id = self.as_mut().rust_mut().notes.clip(
            &selection.to_string(),
            &url.to_string(),
            &title.to_string(),
            now(),
        );
        self.notes_changed();
        i32::try_from(id).unwrap_or(-1)
    }

    fn set_note_text(mut self: Pin<&mut Self>, id: i32, text: &QString) {
        let Ok(id) = u64::try_from(id) else {
            return;
        };
        if self
            .as_mut()
            .rust_mut()
            .notes
            .set_text(id, &text.to_string(), now())
        {
            self.notes_changed();
        }
    }

    fn remove_note(mut self: Pin<&mut Self>, id: i32) {
        let Ok(id) = u64::try_from(id) else {
            return;
        };
        if self.as_mut().rust_mut().notes.remove(id) {
            self.notes_changed();
        }
    }

    fn note_text(&self, id: i32) -> QString {
        let note = u64::try_from(id).ok().and_then(|id| self.notes.get(id));
        QString::from(note.map_or("", |n| n.text.as_str()))
    }
}
