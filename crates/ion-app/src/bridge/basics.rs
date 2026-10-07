//! `Basics` QML singleton: the words and choices behind permission prompts,
//! page dialogs, find in page, the context menu and the new-tab page, from `ion_basics`.

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

        /// "example.com says", or "Leave this page?" for the unsaved-changes
        /// check. `dialog_type` is `JavaScriptDialogRequest.DialogType`.
        #[qinvokable]
        #[cxx_name = "dialogHeading"]
        fn dialog_heading(self: &Basics, dialog_type: i32, origin: &QUrl) -> QString;

        /// The accepting button's label ("OK", "Leave").
        #[qinvokable]
        #[cxx_name = "dialogAcceptLabel"]
        fn dialog_accept_label(self: &Basics, dialog_type: i32) -> QString;

        /// The dismissing button's label, or empty when there is none.
        #[qinvokable]
        #[cxx_name = "dialogRejectLabel"]
        fn dialog_reject_label(self: &Basics, dialog_type: i32) -> QString;

        /// The body shown for the unsaved-changes check.
        #[qinvokable]
        #[cxx_name = "beforeUnloadMessage"]
        fn before_unload_message(self: &Basics) -> QString;

        /// Whether the `count`-th dialog since the page loaded offers to
        /// block the rest.
        #[qinvokable]
        #[cxx_name = "offerDialogBlock"]
        fn offer_dialog_block(self: &Basics, count: i32) -> bool;

        /// "Sign in to example.com" (or to a proxy).
        #[qinvokable]
        #[cxx_name = "authHeading"]
        fn auth_heading(self: &Basics, proxy: bool, url: &QUrl, proxy_host: &QString) -> QString;

        /// The site's realm and a warning for unencrypted connections.
        #[qinvokable]
        #[cxx_name = "authDetail"]
        fn auth_detail(self: &Basics, realm: &QString, url: &QUrl, proxy: bool) -> QString;

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

        /// The new-tab page's tiles as a JSON array of `{title, url, iconPage, letter}`:
        /// `newTab.shortcuts` from the config, then the most-visited sites
        /// from `history` (`History.search("", n)` output, best first).
        #[qinvokable]
        #[cxx_name = "newTabTiles"]
        fn new_tab_tiles(self: &Basics, history: &QString) -> QString;
    }
}

use cxx_qt_lib::{QString, QStringList, QUrl};
use ion_basics::new_tab::{self, Shortcut};
use ion_basics::{context_menu, dialogs, find, permissions};

#[derive(Default)]
pub struct BasicsRust;

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

    fn dialog_heading(&self, dialog_type: i32, origin: &QUrl) -> QString {
        let kind = dialogs::Kind::from_qt(dialog_type).unwrap_or(dialogs::Kind::Alert);
        QString::from(dialogs::heading(kind, &origin.to_string()).as_str())
    }

    fn dialog_accept_label(&self, dialog_type: i32) -> QString {
        let kind = dialogs::Kind::from_qt(dialog_type).unwrap_or(dialogs::Kind::Alert);
        QString::from(kind.accept_label())
    }

    fn dialog_reject_label(&self, dialog_type: i32) -> QString {
        dialogs::Kind::from_qt(dialog_type)
            .and_then(dialogs::Kind::reject_label)
            .map(QString::from)
            .unwrap_or_default()
    }

    fn before_unload_message(&self) -> QString {
        QString::from(dialogs::BEFORE_UNLOAD_MESSAGE)
    }

    fn offer_dialog_block(&self, count: i32) -> bool {
        dialogs::offer_block(count.max(0) as u32)
    }

    fn auth_heading(&self, proxy: bool, url: &QUrl, proxy_host: &QString) -> QString {
        let heading = dialogs::auth_heading(proxy, &url.to_string(), &proxy_host.to_string());
        QString::from(heading.as_str())
    }

    fn auth_detail(&self, realm: &QString, url: &QUrl, proxy: bool) -> QString {
        let detail = dialogs::auth_detail(&realm.to_string(), &url.to_string(), proxy);
        QString::from(detail.as_str())
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

    fn new_tab_tiles(&self, history: &QString) -> QString {
        let config = ion_config::global().config();
        let pinned: Vec<Shortcut> = config
            .new_tab
            .shortcuts
            .iter()
            .map(|s| Shortcut::new(&s.title, &s.url))
            .collect();
        let visited: Vec<Shortcut> =
            serde_json::from_str::<Vec<serde_json::Value>>(&history.to_string())
                .unwrap_or_default()
                .iter()
                .map(|page| {
                    let field = |key: &str| page.get(key).and_then(|v| v.as_str()).unwrap_or("");
                    Shortcut::new(field("title"), field("url"))
                })
                .collect();
        let max = usize::try_from(config.new_tab.tiles).unwrap_or(usize::MAX);
        let tiles: Vec<serde_json::Value> =
            new_tab::tiles(&pinned, &visited, config.new_tab.most_visited, max)
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "title": t.title,
                        "url": t.url,
                        "iconPage": t.icon_page,
                        "letter": t.letter(),
                    })
                })
                .collect();
        QString::from(serde_json::Value::Array(tiles).to_string().as_str())
    }
}
