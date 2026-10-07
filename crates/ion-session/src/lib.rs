//! Tabs, sessions and history for Ion.
//!
//! - [`tabs::TabList`]: the ordered tab list with the current tab, the
//!   workspaces tabs are grouped into, and a stack of recently closed tabs.
//!   The QML tab model in `ion-app` is a thin adapter over it.
//! - [`session`]: snapshots of a tab list, saved on every change so the browser
//!   reopens where it left off, plus named sessions people save and reopen.
//! - [`history::History`]: visited pages, searchable for the command palette.
//!
//! Nothing here knows about Qt; all of it is unit tested on its own.

pub mod history;
pub mod paths;
pub mod session;
pub mod tabs;

pub use history::{History, HistoryEntry};
pub use session::{SavedTab, SavedWorkspace, Session, SessionStore};
pub use tabs::{Tab, TabId, TabList, Workspace, WorkspaceId};
