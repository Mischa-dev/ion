use ion_bookmarks::{Bookmark, Bookmarks};

fn bookmark(url: &str, title: &str, folder: &str, added: u64) -> Bookmark {
    Bookmark {
        url: url.into(),
        title: title.into(),
        folder: folder.into(),
        added,
    }
}

#[test]
fn collection_round_trips_through_disk_without_losing_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/bookmarks.json");
    let mut original = Bookmarks::new();
    original.insert(bookmark(
        "https://www.rust-lang.org/learn",
        "Learn Rust",
        "Development/Languages",
        1_700_000_000,
    ));
    original.insert(bookmark(
        "file:///home/alice/notes.html",
        "Local notes",
        "Personal",
        1_700_000_100,
    ));

    original.save(&path).unwrap();
    let restored = Bookmarks::load(&path).unwrap();

    assert_eq!(
        restored.iter().cloned().collect::<Vec<_>>(),
        original.iter().cloned().collect::<Vec<_>>()
    );
}

#[test]
fn loading_deduplicates_urls_from_an_external_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bookmarks.json");
    std::fs::write(
        &path,
        r#"{"bookmarks":[
          {"url":"https://example.com/","title":"First","added":1},
          {"url":"https://example.com/","title":"Duplicate","added":2},
          {"url":"about:blank","title":"Internal","added":3}
        ]}"#,
    )
    .unwrap();

    let restored = Bookmarks::load(&path).unwrap();

    assert_eq!(restored.len(), 1);
    assert_eq!(restored.iter().next().unwrap().title, "First");
}

#[test]
fn malformed_web_addresses_are_rejected_but_valid_schemes_ignore_case() {
    let mut bookmarks = Bookmarks::new();

    assert!(!bookmarks.insert(bookmark("https://", "No host", "", 1)));
    assert!(!bookmarks.insert(bookmark("https://not a host/", "Bad host", "", 2)));
    assert!(bookmarks.insert(bookmark(
        "HTTPS://example.com/",
        "Valid uppercase scheme",
        "",
        3
    )));
    assert_eq!(bookmarks.len(), 1);
}

#[test]
fn multi_word_search_uses_all_fields_and_respects_limit() {
    let mut bookmarks = Bookmarks::new();
    bookmarks.extend([
        bookmark(
            "https://doc.rust-lang.org/book/",
            "The Rust Book",
            "Coding",
            10,
        ),
        bookmark(
            "https://example.test/rust",
            "Language notes",
            "Coding/Books",
            20,
        ),
        bookmark("https://example.test/other", "Rust news", "Reading", 30),
    ]);

    let matches = bookmarks.search("rust book", 1);

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].title, "The Rust Book");
}
