//! The page context menu: which entries to show for what was clicked.
//!
//! QML turns each [`Item`] id into a label and an action, so the wording stays
//! translatable there while the choice of entries is decided (and tested) here.

/// One context-menu entry. `Separator` draws a line between groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Back,
    Forward,
    Reload,
    OpenLinkInNewTab,
    CopyLink,
    SaveLink,
    OpenImageInNewTab,
    CopyImage,
    CopyImageAddress,
    SaveImage,
    PlayPause,
    Mute,
    Loop,
    ShowControls,
    CopyMediaAddress,
    SaveMedia,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    PasteAsPlainText,
    SelectAll,
    SearchSelection,
    SavePage,
    Screenshot,
    ViewSource,
    Separator,
}

impl Item {
    /// Stable id QML switches on.
    pub fn id(self) -> &'static str {
        match self {
            Item::Back => "back",
            Item::Forward => "forward",
            Item::Reload => "reload",
            Item::OpenLinkInNewTab => "openLinkInNewTab",
            Item::CopyLink => "copyLink",
            Item::SaveLink => "saveLink",
            Item::OpenImageInNewTab => "openImageInNewTab",
            Item::CopyImage => "copyImage",
            Item::CopyImageAddress => "copyImageAddress",
            Item::SaveImage => "saveImage",
            Item::PlayPause => "playPause",
            Item::Mute => "mute",
            Item::Loop => "loop",
            Item::ShowControls => "showControls",
            Item::CopyMediaAddress => "copyMediaAddress",
            Item::SaveMedia => "saveMedia",
            Item::Undo => "undo",
            Item::Redo => "redo",
            Item::Cut => "cut",
            Item::Copy => "copy",
            Item::Paste => "paste",
            Item::PasteAsPlainText => "pasteAsPlainText",
            Item::SelectAll => "selectAll",
            Item::SearchSelection => "searchSelection",
            Item::SavePage => "savePage",
            Item::Screenshot => "screenshot",
            Item::ViewSource => "viewSource",
            Item::Separator => "-",
        }
    }
}

/// What was under the pointer, from `WebEngineContextMenuRequest`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Target {
    pub has_link: bool,
    /// `ContextMenuRequest.mediaType` as an integer.
    pub media_type: i32,
    pub media_flags: u32,
    pub editable: bool,
    pub edit_flags: u32,
    pub has_selection: bool,
}

// `QWebEngineContextMenuRequest::MediaType`.
const MEDIA_IMAGE: i32 = 1;
const MEDIA_VIDEO: i32 = 2;
const MEDIA_AUDIO: i32 = 3;
const MEDIA_CANVAS: i32 = 4;

// `QWebEngineContextMenuRequest::MediaFlag`.
const MEDIA_IN_ERROR: u32 = 0x1;
const MEDIA_CAN_SAVE: u32 = 0x10;
const MEDIA_HAS_AUDIO: u32 = 0x20;
const MEDIA_CAN_TOGGLE_CONTROLS: u32 = 0x40;

// `QWebEngineContextMenuRequest::EditFlag`.
const CAN_UNDO: u32 = 0x1;
const CAN_REDO: u32 = 0x2;
const CAN_CUT: u32 = 0x4;
const CAN_COPY: u32 = 0x8;
const CAN_PASTE: u32 = 0x10;
const CAN_SELECT_ALL: u32 = 0x40;

/// The entries for `target`, grouped with separators, most specific first:
/// link, then media, then editing or selection, and page actions only when
/// nothing more specific was clicked.
pub fn items(target: Target) -> Vec<Item> {
    let mut groups: Vec<Vec<Item>> = Vec::new();

    if target.has_link {
        groups.push(vec![Item::OpenLinkInNewTab, Item::CopyLink, Item::SaveLink]);
    }

    match target.media_type {
        MEDIA_IMAGE | MEDIA_CANVAS => {
            let mut group = vec![];
            if target.media_type == MEDIA_IMAGE {
                group.push(Item::OpenImageInNewTab);
            }
            group.push(Item::CopyImage);
            if target.media_type == MEDIA_IMAGE {
                group.push(Item::CopyImageAddress);
            }
            if target.media_flags & MEDIA_CAN_SAVE != 0 {
                group.push(Item::SaveImage);
            }
            groups.push(group);
        }
        MEDIA_VIDEO | MEDIA_AUDIO if target.media_flags & MEDIA_IN_ERROR == 0 => {
            let mut group = vec![Item::PlayPause];
            if target.media_type == MEDIA_AUDIO || target.media_flags & MEDIA_HAS_AUDIO != 0 {
                group.push(Item::Mute);
            }
            group.push(Item::Loop);
            if target.media_flags & MEDIA_CAN_TOGGLE_CONTROLS != 0 {
                group.push(Item::ShowControls);
            }
            group.push(Item::CopyMediaAddress);
            if target.media_flags & MEDIA_CAN_SAVE != 0 {
                group.push(Item::SaveMedia);
            }
            groups.push(group);
        }
        _ => {}
    }

    if target.editable {
        let flag = |f: u32| target.edit_flags & f != 0;
        let mut history = vec![];
        if flag(CAN_UNDO) {
            history.push(Item::Undo);
        }
        if flag(CAN_REDO) {
            history.push(Item::Redo);
        }
        groups.push(history);
        let mut clipboard = vec![];
        if flag(CAN_CUT) {
            clipboard.push(Item::Cut);
        }
        if flag(CAN_COPY) {
            clipboard.push(Item::Copy);
        }
        if flag(CAN_PASTE) {
            clipboard.push(Item::Paste);
            clipboard.push(Item::PasteAsPlainText);
        }
        if flag(CAN_SELECT_ALL) {
            clipboard.push(Item::SelectAll);
        }
        groups.push(clipboard);
        if target.has_selection {
            groups.push(vec![Item::SearchSelection]);
        }
    } else if target.has_selection {
        groups.push(vec![Item::Copy, Item::SearchSelection]);
    }

    // Page actions only when nothing more specific was clicked.
    let clicked_something = target.editable || groups.iter().any(|g| !g.is_empty());
    if !clicked_something {
        groups.push(vec![Item::Back, Item::Forward, Item::Reload]);
        groups.push(vec![Item::SavePage, Item::Screenshot, Item::ViewSource]);
    }

    let mut out = Vec::new();
    for group in groups.into_iter().filter(|g| !g.is_empty()) {
        if !out.is_empty() {
            out.push(Item::Separator);
        }
        out.extend(group);
    }
    out
}

/// The text "Search for …" shows: the selection, collapsed to one line and
/// shortened so the menu stays narrow.
pub fn selection_preview(selection: &str) -> String {
    const MAX_CHARS: usize = 32;
    let one_line = selection.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= MAX_CHARS {
        one_line
    } else {
        let cut: String = one_line.chars().take(MAX_CHARS - 1).collect();
        format!("{}…", cut.trim_end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(target: Target) -> Vec<&'static str> {
        items(target).into_iter().map(Item::id).collect()
    }

    #[test]
    fn plain_page() {
        assert_eq!(
            ids(Target::default()),
            [
                "back",
                "forward",
                "reload",
                "-",
                "savePage",
                "screenshot",
                "viewSource"
            ]
        );
    }

    #[test]
    fn link_hides_page_actions() {
        let target = Target {
            has_link: true,
            ..Default::default()
        };
        assert_eq!(ids(target), ["openLinkInNewTab", "copyLink", "saveLink"]);
    }

    #[test]
    fn linked_image() {
        let target = Target {
            has_link: true,
            media_type: MEDIA_IMAGE,
            media_flags: MEDIA_CAN_SAVE,
            ..Default::default()
        };
        assert_eq!(
            ids(target),
            [
                "openLinkInNewTab",
                "copyLink",
                "saveLink",
                "-",
                "openImageInNewTab",
                "copyImage",
                "copyImageAddress",
                "saveImage"
            ]
        );
    }

    #[test]
    fn video_with_audio() {
        let target = Target {
            media_type: MEDIA_VIDEO,
            media_flags: MEDIA_HAS_AUDIO | MEDIA_CAN_SAVE | MEDIA_CAN_TOGGLE_CONTROLS,
            ..Default::default()
        };
        assert_eq!(
            ids(target),
            [
                "playPause",
                "mute",
                "loop",
                "showControls",
                "copyMediaAddress",
                "saveMedia"
            ]
        );
    }

    #[test]
    fn broken_video_gets_page_actions_only() {
        let target = Target {
            media_type: MEDIA_VIDEO,
            media_flags: MEDIA_IN_ERROR,
            ..Default::default()
        };
        assert_eq!(ids(target).first(), Some(&"back"));
    }

    #[test]
    fn editable_field_with_selection() {
        let target = Target {
            editable: true,
            edit_flags: CAN_UNDO | CAN_CUT | CAN_COPY | CAN_PASTE | CAN_SELECT_ALL,
            has_selection: true,
            ..Default::default()
        };
        assert_eq!(
            ids(target),
            [
                "undo",
                "-",
                "cut",
                "copy",
                "paste",
                "pasteAsPlainText",
                "selectAll",
                "-",
                "searchSelection"
            ]
        );
    }

    #[test]
    fn empty_editable_field_skips_empty_groups() {
        let target = Target {
            editable: true,
            edit_flags: CAN_PASTE,
            ..Default::default()
        };
        assert_eq!(ids(target), ["paste", "pasteAsPlainText"]);
    }

    #[test]
    fn selected_text() {
        let target = Target {
            has_selection: true,
            ..Default::default()
        };
        assert_eq!(ids(target), ["copy", "searchSelection"]);
    }

    #[test]
    fn selection_preview_shortens() {
        assert_eq!(selection_preview("  hello\n world "), "hello world");
        let long = "a".repeat(40);
        let preview = selection_preview(&long);
        assert_eq!(preview.chars().count(), 32);
        assert!(preview.ends_with('…'));
    }
}
