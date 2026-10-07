//! `Basics` QML singleton: the words and choices behind permission prompts,
//! find in page, the context menu and the new-tab page, from `ion_basics`.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(i32, shortcut_count, cxx_name = "shortcutCount")]
        #[namespace = "ion"]
        type Basics = super::BasicsRust;

        /// "example.com wants to use your camera", or empty for permission
        /// types Ion does not prompt for (deny those).
        #[qinvokable]
        #[cxx_name = "permissionText"]
        fn permission_text(self: &Basics, permission_type: i32, origin: &QUrl) -> QString;

        /// Glyph for a permission type's prompt.
        #[qinvokable]
        #[cxx_name = "permissionGlyph"]
        fn permission_glyph(self: &Basics, permission_type: i32) -> QString;

        /// "3 of 12", "No matches", or empty.
        #[qinvokable]
        #[cxx_name = "findLabel"]
        fn find_label(self: &Basics, query: &QString, active: i32, total: i32) -> QString;

        /// Context menu entry ids for what was clicked; "-" is a separator.
        #[qinvokable]
        #[cxx_name = "contextMenuItems"]
        fn context_menu_items(
            self: &Basics,
            has_link: bool,
            media_type: i32,
            media_flags: i32,
            editable: bool,
            edit_flags: i32,
            has_selection: bool,
        ) -> QStringList;

        /// Selected text shortened for "Search for …".
        #[qinvokable]
        #[cxx_name = "selectionPreview"]
        fn selection_preview(self: &Basics, text: &QString) -> QString;

        /// Greeting for the new-tab page at `hour` (0–23).
        #[qinvokable]
        fn greeting(self: &Basics, hour: i32) -> QString;

        /// True if `url` shows the new-tab page.
        #[qinvokable]
        #[cxx_name = "isNewTabUrl"]
        fn is_new_tab_url(self: &Basics, url: &QUrl) -> bool;

        #[qinvokable]
        #[cxx_name = "shortcutTitle"]
        fn shortcut_title(self: &Basics, index: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "shortcutUrl"]
        fn shortcut_url(self: &Basics, index: i32) -> QUrl;

        #[qinvokable]
        #[cxx_name = "shortcutLetter"]
        fn shortcut_letter(self: &Basics, index: i32) -> QString;
    }
}

use cxx_qt_lib::{QString, QStringList, QUrl};
use ion_basics::new_tab::{self, Shortcut};
use ion_basics::{context_menu, find, permissions};

pub struct BasicsRust {
    shortcut_count: i32,
    shortcuts: Vec<Shortcut>,
}

impl Default for BasicsRust {
    fn default() -> Self {
        let shortcuts = new_tab::default_shortcuts();
        Self {
            shortcut_count: i32::try_from(shortcuts.len()).unwrap_or(i32::MAX),
            shortcuts,
        }
    }
}

impl BasicsRust {
    fn shortcut(&self, index: i32) -> Option<&Shortcut> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.shortcuts.get(i))
    }
}

fn flags(value: i32) -> u32 {
    u32::try_from(value).unwrap_or(0)
}

impl qobject::Basics {
    fn permission_text(&self, permission_type: i32, origin: &QUrl) -> QString {
        match permissions::Kind::from_qt(permission_type) {
            Some(kind) => {
                QString::from(permissions::prompt_text(kind, &origin.to_string()).as_str())
            }
            None => QString::default(),
        }
    }

    fn permission_glyph(&self, permission_type: i32) -> QString {
        permissions::Kind::from_qt(permission_type)
            .map(|kind| QString::from(kind.glyph()))
            .unwrap_or_default()
    }

    fn find_label(&self, query: &QString, active: i32, total: i32) -> QString {
        let label = find::match_label(
            &query.to_string(),
            u32::try_from(active).unwrap_or(0),
            u32::try_from(total).unwrap_or(0),
        );
        QString::from(label.as_str())
    }

    fn context_menu_items(
        &self,
        has_link: bool,
        media_type: i32,
        media_flags: i32,
        editable: bool,
        edit_flags: i32,
        has_selection: bool,
    ) -> QStringList {
        let target = context_menu::Target {
            has_link,
            media_type,
            media_flags: flags(media_flags),
            editable,
            edit_flags: flags(edit_flags),
            has_selection,
        };
        context_menu::items(target)
            .into_iter()
            .map(|item| QString::from(item.id()))
            .collect()
    }

    fn selection_preview(&self, text: &QString) -> QString {
        QString::from(context_menu::selection_preview(&text.to_string()).as_str())
    }

    fn greeting(&self, hour: i32) -> QString {
        QString::from(new_tab::greeting(u32::try_from(hour).unwrap_or(0)))
    }

    fn is_new_tab_url(&self, url: &QUrl) -> bool {
        new_tab::is_new_tab_url(&url.to_string())
    }

    fn shortcut_title(&self, index: i32) -> QString {
        self.shortcut(index)
            .map(|s| QString::from(s.title.as_str()))
            .unwrap_or_default()
    }

    fn shortcut_url(&self, index: i32) -> QUrl {
        self.shortcut(index)
            .map(|s| QUrl::from(s.url.as_str()))
            .unwrap_or_default()
    }

    fn shortcut_letter(&self, index: i32) -> QString {
        self.shortcut(index)
            .map(|s| QString::from(s.letter().as_str()))
            .unwrap_or_default()
    }
}
