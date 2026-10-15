use std::sync::Arc;

use ion_bangs::{Action, BangTable, Kind, Palette, Sources, TabEntry};
use ion_core::navigation::{Omnibox, Resolved, SearchEngine};

fn browser() -> (Arc<BangTable>, Omnibox, Palette) {
    let bangs = Arc::new(BangTable::default());
    let omnibox = Omnibox::new(SearchEngine::new(
        "Test Search",
        "https://search.example/?q={}",
    ))
    .with_step(bangs.clone());
    let palette = Palette::new(omnibox.clone(), bangs.clone());
    (bangs, omnibox, palette)
}

#[test]
fn configured_bang_runs_before_normal_url_resolution() {
    let (bangs, omnibox, _) = browser();

    assert_eq!(
        omnibox.resolve("github.com !gh"),
        Some(Resolved::Expanded(
            "https://github.com/search?q=github.com".into()
        ))
    );
    assert!(bangs.get("GH").is_some());
}

#[test]
fn unknown_bang_falls_back_to_the_search_provider() {
    let (_, omnibox, _) = browser();

    assert_eq!(
        omnibox.resolve("!missing rust & safety"),
        Some(Resolved::Search(
            "https://search.example/?q=%21missing+rust+%26+safety".into()
        ))
    );
}

#[test]
fn selecting_a_bang_palette_result_opens_the_same_url_as_the_omnibox() {
    let (_, omnibox, palette) = browser();
    let input = "!rs url crate";
    let results = palette.query(input, &Sources::default());

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].kind, Kind::Bang);
    assert_eq!(
        results[0].action,
        Action::Open(omnibox.resolve(input).unwrap().url().to_owned())
    );
}

#[test]
fn suggestions_keep_the_enter_action_first_and_deduplicate_open_pages() {
    let (_, _, palette) = browser();
    let tabs = [TabEntry {
        title: "Rust language".into(),
        url: "https://www.rust-lang.org/".into(),
    }];
    let history = [
        tabs[0].clone(),
        TabEntry {
            title: "Rust book".into(),
            url: "https://doc.rust-lang.org/book/".into(),
        },
    ];
    let sources = Sources {
        tabs: &tabs,
        history: &history,
        ..Sources::default()
    };

    let results = palette.suggest("rust", &sources);

    assert_eq!(results[0].kind, Kind::Search);
    assert_eq!(
        results.iter().filter(|item| item.kind == Kind::Tab).count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|item| item.kind == Kind::History)
            .count(),
        1
    );
}

#[test]
fn bookmark_prefix_returns_only_bookmarks() {
    let (_, _, palette) = browser();
    let bookmarks = [TabEntry {
        title: "Rust documentation".into(),
        url: "https://doc.rust-lang.org/".into(),
    }];
    let tabs = [TabEntry {
        title: "Rust playground".into(),
        url: "https://play.rust-lang.org/".into(),
    }];
    let results = palette.query(
        "* rust",
        &Sources {
            tabs: &tabs,
            bookmarks: &bookmarks,
            ..Sources::default()
        },
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].kind, Kind::Bookmark);
    assert_eq!(results[0].action, Action::Open(bookmarks[0].url.clone()));
}
