//! `Tabs` QML singleton: the open tabs as a list model, backed by
//! `ion_session::TabList`, saved to disk so Ion reopens where it left off.
//!
//! Roles: `tabId` (stable while the tab is open), `url`, `title`. The window
//! creates one web view per row and reports navigation back with `setUrl` and
//! `setTitle`; everything else (open, close, reorder, sessions) goes through the
//! invokables below so the strip, the palette and later agents share one model.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!(<QtCore/QAbstractListModel>);
        type QAbstractListModel;

        include!("cxx-qt-lib/qbytearray.h");
        type QByteArray = cxx_qt_lib::QByteArray;
        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
        include!("cxx-qt-lib/qlist.h");
        type QList_i32 = cxx_qt_lib::QList<i32>;
        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    extern "RustQt" {
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(i32, count, READ, NOTIFY)]
        #[qproperty(i32, current_index, cxx_name = "currentIndex", READ, NOTIFY)]
        #[qproperty(i32, closed_count, cxx_name = "closedCount", READ, NOTIFY)]
        #[qproperty(bool, restoring, READ, NOTIFY)]
        #[namespace = "ion"]
        type Tabs = super::TabsRust;
    }

    // QAbstractListModel interface.
    unsafe extern "RustQt" {
        #[qinvokable]
        #[cxx_override]
        fn data(self: &Tabs, index: &QModelIndex, role: i32) -> QVariant;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &Tabs) -> QHash_i32_QByteArray;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &Tabs, parent: &QModelIndex) -> i32;
    }

    unsafe extern "RustQt" {
        #[inherit]
        #[cxx_name = "beginInsertRows"]
        fn begin_insert_rows(self: Pin<&mut Tabs>, parent: &QModelIndex, first: i32, last: i32);
        #[inherit]
        #[cxx_name = "endInsertRows"]
        fn end_insert_rows(self: Pin<&mut Tabs>);
        #[inherit]
        #[cxx_name = "beginRemoveRows"]
        fn begin_remove_rows(self: Pin<&mut Tabs>, parent: &QModelIndex, first: i32, last: i32);
        #[inherit]
        #[cxx_name = "endRemoveRows"]
        fn end_remove_rows(self: Pin<&mut Tabs>);
        #[inherit]
        #[cxx_name = "beginMoveRows"]
        fn begin_move_rows(
            self: Pin<&mut Tabs>,
            source_parent: &QModelIndex,
            source_first: i32,
            source_last: i32,
            destination_parent: &QModelIndex,
            destination_child: i32,
        ) -> bool;
        #[inherit]
        #[cxx_name = "endMoveRows"]
        fn end_move_rows(self: Pin<&mut Tabs>);
        #[inherit]
        #[cxx_name = "beginResetModel"]
        fn begin_reset_model(self: Pin<&mut Tabs>);
        #[inherit]
        #[cxx_name = "endResetModel"]
        fn end_reset_model(self: Pin<&mut Tabs>);
        #[inherit]
        fn index(self: &Tabs, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;

        #[qsignal]
        #[inherit]
        #[cxx_name = "dataChanged"]
        fn data_changed(
            self: Pin<&mut Tabs>,
            top_left: &QModelIndex,
            bottom_right: &QModelIndex,
            roles: &QList_i32,
        );
    }

    extern "RustQt" {
        /// Something worth saving changed (tabs, their order, URLs or titles).
        #[qsignal]
        #[cxx_name = "sessionChanged"]
        fn session_changed(self: Pin<&mut Tabs>);

        /// Open a tab at the end of the strip. Returns its index.
        #[qinvokable]
        #[cxx_name = "openTab"]
        fn open_tab(self: Pin<&mut Tabs>, url: &QString, activate: bool) -> i32;

        /// Open a tab right after the current one (links opened from a page).
        #[qinvokable]
        #[cxx_name = "openTabNextToCurrent"]
        fn open_tab_next_to_current(self: Pin<&mut Tabs>, url: &QString, activate: bool) -> i32;

        /// Open a tab at `index` (clamped to the end of the strip). Returns its
        /// index.
        #[qinvokable]
        #[cxx_name = "openTabAt"]
        fn open_tab_at(self: Pin<&mut Tabs>, index: i32, url: &QString, activate: bool) -> i32;

        /// Close the tab at `index`. False if there is no such tab.
        #[qinvokable]
        #[cxx_name = "closeTab"]
        fn close_tab(self: Pin<&mut Tabs>, index: i32) -> bool;

        /// Reopen the most recently closed tab. Returns its index, or -1.
        #[qinvokable]
        #[cxx_name = "reopenClosedTab"]
        fn reopen_closed_tab(self: Pin<&mut Tabs>) -> i32;

        #[qinvokable]
        fn activate(self: Pin<&mut Tabs>, index: i32);

        /// Activate the tab `step` places away, wrapping around.
        #[qinvokable]
        fn cycle(self: Pin<&mut Tabs>, step: i32);

        /// Move the tab at `from` so it ends up at `to`.
        #[qinvokable]
        #[cxx_name = "moveTab"]
        fn move_tab(self: Pin<&mut Tabs>, from: i32, to: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "setUrl"]
        fn set_url(self: Pin<&mut Tabs>, index: i32, url: &QString);

        #[qinvokable]
        #[cxx_name = "setTitle"]
        fn set_title(self: Pin<&mut Tabs>, index: i32, title: &QString);

        /// The tab's favicon URL as its web view reports it.
        #[qinvokable]
        #[cxx_name = "setIcon"]
        fn set_icon(self: Pin<&mut Tabs>, index: i32, icon: &QString);

        /// Background tabs not shown for `seconds` and not suspended yet.
        #[qinvokable]
        #[cxx_name = "idleTabs"]
        fn idle_tabs(self: &Tabs, seconds: i32) -> QList_i32;

        /// Suspend (unload) or wake a background tab. The current tab is
        /// always awake.
        #[qinvokable]
        #[cxx_name = "setSuspended"]
        fn set_suspended(self: Pin<&mut Tabs>, index: i32, suspended: bool);

        #[qinvokable]
        #[cxx_name = "urlAt"]
        fn url_at(self: &Tabs, index: i32) -> QString;

        #[qinvokable]
        #[cxx_name = "titleAt"]
        fn title_at(self: &Tabs, index: i32) -> QString;

        /// Replace the tabs with the ones open when Ion last quit. False if
        /// there was nothing to restore.
        #[qinvokable]
        #[cxx_name = "restoreLastSession"]
        fn restore_last_session(self: Pin<&mut Tabs>) -> bool;

        /// Write the open tabs to disk for the next start.
        #[qinvokable]
        #[cxx_name = "saveSession"]
        fn save_session(self: &Tabs) -> bool;

        /// Save the open tabs as a named session.
        #[qinvokable]
        #[cxx_name = "saveSessionAs"]
        fn save_session_as(self: &Tabs, name: &QString) -> bool;

        /// Names of the saved sessions, sorted.
        #[qinvokable]
        #[cxx_name = "sessionNames"]
        fn session_names(self: &Tabs) -> QStringList;

        /// Replace the open tabs with a named session.
        #[qinvokable]
        #[cxx_name = "openSession"]
        fn open_session(self: Pin<&mut Tabs>, name: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "deleteSession"]
        fn delete_session(self: &Tabs, name: &QString) -> bool;
    }
}

use core::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{
    QByteArray, QHash, QHashPair_i32_QByteArray, QList, QModelIndex, QString, QStringList, QVariant,
};
use ion_session::history::now;
use ion_session::{Session, SessionStore, TabList};

const ROLE_ID: i32 = 0x0100; // Qt::UserRole
const ROLE_URL: i32 = ROLE_ID + 1;
const ROLE_TITLE: i32 = ROLE_ID + 2;
const ROLE_ICON: i32 = ROLE_ID + 3;
const ROLE_SUSPENDED: i32 = ROLE_ID + 4;

pub struct TabsRust {
    list: TabList,
    store: Option<SessionStore>,
    count: i32,
    current_index: i32,
    closed_count: i32,
    restoring: bool,
}

impl Default for TabsRust {
    fn default() -> Self {
        Self {
            list: TabList::new(),
            store: SessionStore::in_data_dir(),
            count: 0,
            current_index: -1,
            closed_count: 0,
            restoring: false,
        }
    }
}

fn to_index(index: i32, len: usize) -> Option<usize> {
    usize::try_from(index).ok().filter(|&i| i < len)
}

fn warn(what: &str, err: impl std::fmt::Display) {
    eprintln!("ion: {what}: {err}");
}

impl qobject::Tabs {
    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let Some(tab) = to_index(index.row(), self.list.len()).and_then(|i| self.list.get(i))
        else {
            return QVariant::default();
        };
        match role {
            ROLE_ID => QVariant::from(&(tab.id as i32)),
            ROLE_URL => QVariant::from(&QString::from(tab.url.as_str())),
            ROLE_TITLE => QVariant::from(&QString::from(tab.title.as_str())),
            ROLE_ICON => QVariant::from(&QString::from(tab.icon.as_str())),
            ROLE_SUSPENDED => QVariant::from(&tab.suspended),
            _ => QVariant::default(),
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        roles.insert(ROLE_ID, QByteArray::from("tabId"));
        roles.insert(ROLE_URL, QByteArray::from("url"));
        roles.insert(ROLE_TITLE, QByteArray::from("title"));
        roles.insert(ROLE_ICON, QByteArray::from("icon"));
        roles.insert(ROLE_SUSPENDED, QByteArray::from("suspended"));
        roles
    }

    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        self.list.len() as i32
    }

    /// Push count, current index and closed count to QML after a change.
    fn sync(mut self: Pin<&mut Self>) {
        // Wake the current tab before announcing it, so its view is never shown
        // while still unloaded.
        if let Some(woken) = self.as_mut().rust_mut().list.note_current(now()) {
            self.as_mut().notify_row(woken as i32, ROLE_SUSPENDED);
        }
        let count = self.list.len() as i32;
        let current = self.list.current().map_or(-1, |i| i as i32);
        let closed = self.list.closed_count() as i32;
        if self.count != count {
            self.as_mut().rust_mut().count = count;
            self.as_mut().count_changed();
        }
        if self.current_index != current {
            self.as_mut().rust_mut().current_index = current;
            self.as_mut().current_index_changed();
        }
        if self.closed_count != closed {
            self.as_mut().rust_mut().closed_count = closed;
            self.as_mut().closed_count_changed();
        }
        self.session_changed();
    }

    fn insert_at(mut self: Pin<&mut Self>, index: usize, url: &QString, activate: bool) -> i32 {
        let index = index.min(self.list.len());
        let row = index as i32;
        self.as_mut()
            .begin_insert_rows(&QModelIndex::default(), row, row);
        self.as_mut()
            .rust_mut()
            .list
            .open(index, &url.to_string(), "", activate);
        self.as_mut().end_insert_rows();
        self.sync();
        row
    }

    fn open_tab(self: Pin<&mut Self>, url: &QString, activate: bool) -> i32 {
        let end = self.list.len();
        self.insert_at(end, url, activate)
    }

    fn open_tab_next_to_current(self: Pin<&mut Self>, url: &QString, activate: bool) -> i32 {
        let next = self.list.current().map_or(0, |i| i + 1);
        self.insert_at(next, url, activate)
    }

    fn open_tab_at(self: Pin<&mut Self>, index: i32, url: &QString, activate: bool) -> i32 {
        self.insert_at(index.max(0) as usize, url, activate)
    }

    fn close_tab(mut self: Pin<&mut Self>, index: i32) -> bool {
        let Some(i) = to_index(index, self.list.len()) else {
            return false;
        };
        self.as_mut()
            .begin_remove_rows(&QModelIndex::default(), index, index);
        if let Some(tab) = self.as_mut().rust_mut().list.close(i) {
            // Prompts and tab-scoped rules for the tab end with it.
            ion_safety::global().tab_closed(tab.id);
        }
        self.as_mut().end_remove_rows();
        self.sync();
        true
    }

    fn reopen_closed_tab(mut self: Pin<&mut Self>) -> i32 {
        let Some(index) = self.list.next_reopen_index() else {
            return -1;
        };
        let row = index as i32;
        self.as_mut()
            .begin_insert_rows(&QModelIndex::default(), row, row);
        self.as_mut().rust_mut().list.reopen_closed();
        self.as_mut().end_insert_rows();
        self.sync();
        row
    }

    fn activate(mut self: Pin<&mut Self>, index: i32) {
        let Some(i) = to_index(index, self.list.len()) else {
            return;
        };
        if self.as_mut().rust_mut().list.activate(i) {
            self.sync();
        }
    }

    fn cycle(mut self: Pin<&mut Self>, step: i32) {
        if self.as_mut().rust_mut().list.cycle(step as isize).is_some() {
            self.sync();
        }
    }

    fn move_tab(mut self: Pin<&mut Self>, from: i32, to: i32) -> bool {
        let len = self.list.len();
        let (Some(f), Some(t)) = (to_index(from, len), to_index(to, len)) else {
            return false;
        };
        if f == t {
            return false;
        }
        // Qt wants the destination as "insert before this row" in the old layout.
        let destination = if t > f { to + 1 } else { to };
        let root = QModelIndex::default();
        if !self
            .as_mut()
            .begin_move_rows(&root, from, from, &root, destination)
        {
            return false;
        }
        self.as_mut().rust_mut().list.move_tab(f, t);
        self.as_mut().end_move_rows();
        self.sync();
        true
    }

    fn notify_row(mut self: Pin<&mut Self>, row: i32, role: i32) {
        let index = self.index(row, 0, &QModelIndex::default());
        let mut roles = QList::<i32>::default();
        roles.append(role);
        self.as_mut().data_changed(&index, &index, &roles);
        self.session_changed();
    }

    fn set_url(mut self: Pin<&mut Self>, index: i32, url: &QString) {
        let Some(i) = to_index(index, self.list.len()) else {
            return;
        };
        if self.as_mut().rust_mut().list.set_url(i, &url.to_string()) {
            self.notify_row(index, ROLE_URL);
        }
    }

    fn set_title(mut self: Pin<&mut Self>, index: i32, title: &QString) {
        let Some(i) = to_index(index, self.list.len()) else {
            return;
        };
        if self
            .as_mut()
            .rust_mut()
            .list
            .set_title(i, &title.to_string())
        {
            self.notify_row(index, ROLE_TITLE);
        }
    }

    fn set_icon(mut self: Pin<&mut Self>, index: i32, icon: &QString) {
        let Some(i) = to_index(index, self.list.len()) else {
            return;
        };
        if self.as_mut().rust_mut().list.set_icon(i, &icon.to_string()) {
            self.notify_row(index, ROLE_ICON);
        }
    }

    fn idle_tabs(&self, seconds: i32) -> QList<i32> {
        let mut rows = QList::<i32>::default();
        for i in self.list.idle_tabs(now(), seconds.max(0) as u64) {
            rows.append(i as i32);
        }
        rows
    }

    fn set_suspended(mut self: Pin<&mut Self>, index: i32, suspended: bool) {
        let Some(i) = to_index(index, self.list.len()) else {
            return;
        };
        if self.as_mut().rust_mut().list.set_suspended(i, suspended) {
            self.notify_row(index, ROLE_SUSPENDED);
        }
    }

    fn url_at(&self, index: i32) -> QString {
        to_index(index, self.list.len())
            .and_then(|i| self.list.get(i))
            .map_or_else(QString::default, |t| QString::from(t.url.as_str()))
    }

    fn title_at(&self, index: i32) -> QString {
        to_index(index, self.list.len())
            .and_then(|i| self.list.get(i))
            .map_or_else(QString::default, |t| QString::from(t.title.as_str()))
    }

    /// Replace every tab with `session`'s. `restoring` is true while the views
    /// for the new rows are created, so they can defer loading background tabs.
    fn replace_with(mut self: Pin<&mut Self>, session: &Session) {
        self.as_mut().rust_mut().restoring = true;
        self.as_mut().restoring_changed();
        self.as_mut().begin_reset_model();
        let mut safety = ion_safety::global();
        for tab in self.list.tabs() {
            safety.tab_closed(tab.id);
        }
        drop(safety);
        self.as_mut().rust_mut().list = TabList::from_session(session);
        self.as_mut().end_reset_model();
        self.as_mut().rust_mut().restoring = false;
        self.as_mut().restoring_changed();
        self.sync();
    }

    fn restore_last_session(self: Pin<&mut Self>) -> bool {
        let Some(store) = self.store.clone() else {
            return false;
        };
        match store.load_last() {
            Ok(Some(session)) if !session.tabs.is_empty() => {
                self.replace_with(&session);
                true
            }
            Ok(_) => false,
            Err(err) => {
                warn("could not restore the last session", err);
                false
            }
        }
    }

    fn save_session(&self) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        store
            .save_last(&self.list.to_session())
            .map_err(|err| warn("could not save the session", err))
            .is_ok()
    }

    fn save_session_as(&self, name: &QString) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        store
            .save_named(&name.to_string(), &self.list.to_session())
            .map_err(|err| warn("could not save the named session", err))
            .is_ok()
    }

    fn session_names(&self) -> QStringList {
        let names = self
            .store
            .as_ref()
            .map(|s| s.list_named().unwrap_or_default())
            .unwrap_or_default();
        names.iter().map(|n| QString::from(n.as_str())).collect()
    }

    fn open_session(self: Pin<&mut Self>, name: &QString) -> bool {
        let Some(store) = self.store.clone() else {
            return false;
        };
        match store.load_named(&name.to_string()) {
            Ok(Some(session)) if !session.tabs.is_empty() => {
                self.replace_with(&session);
                true
            }
            Ok(_) => false,
            Err(err) => {
                warn("could not open the session", err);
                false
            }
        }
    }

    fn delete_session(&self, name: &QString) -> bool {
        let Some(store) = &self.store else {
            return false;
        };
        store
            .delete_named(&name.to_string())
            .map_err(|err| warn("could not delete the session", err))
            .unwrap_or(false)
    }
}
