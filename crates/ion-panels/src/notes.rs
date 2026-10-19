//! Quick notes and clippings, kept in the side panel.
//!
//! A note is plain text. Clipping from a page makes a note that quotes the
//! selection and remembers the page it came from. Notes are listed most
//! recently changed first.

use serde::{Deserialize, Serialize};

/// One note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: u64,
    pub text: String,
    /// The page a clipping came from; empty for a note typed from scratch.
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub page_title: String,
    /// Seconds since the Unix epoch.
    pub created: u64,
    pub updated: u64,
}

impl Note {
    /// The first non-empty line, as the note's title.
    pub fn title(&self) -> &str {
        self.text
            .lines()
            .map(|l| l.trim().trim_start_matches(['#', '>']).trim())
            .find(|l| !l.is_empty())
            .unwrap_or("")
    }
}

/// Every note.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Notes {
    notes: Vec<Note>,
    next_id: u64,
}

impl Notes {
    /// Read from JSON; anything unreadable starts afresh.
    pub fn from_json(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or_default()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    fn next(&mut self) -> u64 {
        let used = self.notes.iter().map(|n| n.id).max().unwrap_or(0);
        self.next_id = self.next_id.max(used) + 1;
        self.next_id
    }

    /// A new note with `text`; returns its id.
    pub fn add(&mut self, text: &str, now: u64) -> u64 {
        let id = self.next();
        self.notes.push(Note {
            id,
            text: text.to_owned(),
            url: String::new(),
            page_title: String::new(),
            created: now,
            updated: now,
        });
        id
    }

    /// A note quoting `selection` from the page at `url`. Clipping more from
    /// the same page within ten minutes adds to that note instead.
    pub fn clip(&mut self, selection: &str, url: &str, page_title: &str, now: u64) -> u64 {
        const MERGE_SECS: u64 = 10 * 60;
        let quote: String = selection
            .trim()
            .lines()
            .map(|line| format!("> {}\n", line.trim_end()))
            .collect();
        if let Some(note) = self
            .notes
            .iter_mut()
            .filter(|n| n.url == url && !url.is_empty())
            .max_by_key(|n| n.updated)
            .filter(|n| now.saturating_sub(n.updated) <= MERGE_SECS)
        {
            if !note.text.ends_with('\n') {
                note.text.push('\n');
            }
            note.text.push('\n');
            note.text.push_str(&quote);
            note.updated = now;
            return note.id;
        }
        let id = self.next();
        let heading = if page_title.trim().is_empty() {
            url
        } else {
            page_title.trim()
        };
        self.notes.push(Note {
            id,
            text: format!("{heading}\n\n{quote}"),
            url: url.to_owned(),
            page_title: page_title.trim().to_owned(),
            created: now,
            updated: now,
        });
        id
    }

    /// Replace a note's text. An emptied note is deleted.
    pub fn set_text(&mut self, id: u64, text: &str, now: u64) -> bool {
        if text.trim().is_empty() {
            return self.remove(id);
        }
        match self.notes.iter_mut().find(|n| n.id == id) {
            Some(note) if note.text != text => {
                note.text = text.to_owned();
                note.updated = now;
                true
            }
            _ => false,
        }
    }

    pub fn remove(&mut self, id: u64) -> bool {
        let before = self.notes.len();
        self.notes.retain(|n| n.id != id);
        self.notes.len() != before
    }

    pub fn get(&self, id: u64) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    /// Notes containing every word of `query` (in their text, title or
    /// page), most recently changed first.
    pub fn search(&self, query: &str) -> Vec<&Note> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let mut found: Vec<&Note> = self
            .notes
            .iter()
            .filter(|note| {
                let haystack =
                    format!("{} {} {}", note.text, note.page_title, note.url).to_lowercase();
                words.iter().all(|w| haystack.contains(w))
            })
            .collect();
        found.sort_by(|a, b| b.updated.cmp(&a.updated).then(b.id.cmp(&a.id)));
        found
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_are_listed_newest_first_and_found_by_words() {
        let mut notes = Notes::default();
        let a = notes.add("Groceries\nmilk, eggs", 10);
        let b = notes.add("Ideas for Ion", 20);
        assert_eq!(
            notes.search("").iter().map(|n| n.id).collect::<Vec<_>>(),
            [b, a]
        );
        assert!(notes.set_text(a, "Groceries\nmilk, eggs, bread", 30));
        assert_eq!(notes.search("")[0].id, a);
        assert_eq!(notes.search("BREAD milk").len(), 1);
        assert!(notes.search("nothing").is_empty());
        assert_eq!(notes.get(a).unwrap().title(), "Groceries");
    }

    #[test]
    fn emptying_a_note_deletes_it() {
        let mut notes = Notes::default();
        let a = notes.add("x", 1);
        assert!(notes.set_text(a, "  \n", 2));
        assert!(notes.is_empty());
    }

    #[test]
    fn clips_quote_the_page_and_merge_while_reading() {
        let mut notes = Notes::default();
        let a = notes.clip(
            "First part\nsecond line",
            "https://a.example/post",
            "A post",
            100,
        );
        let note = notes.get(a).unwrap();
        assert_eq!(note.text, "A post\n\n> First part\n> second line\n");
        assert_eq!(note.title(), "A post");
        assert_eq!(notes.search("a.example").len(), 1);

        let same = notes.clip("More", "https://a.example/post", "A post", 200);
        assert_eq!(same, a);
        assert!(notes.get(a).unwrap().text.ends_with("\n\n> More\n"));

        let later = notes.clip("Again", "https://a.example/post", "A post", 10_000);
        assert_ne!(later, a);
        assert_eq!(notes.len(), 2);
    }

    #[test]
    fn ids_stay_unique_across_saves() {
        let mut notes = Notes::default();
        notes.add("one", 1);
        let two = notes.add("two", 2);
        notes.remove(two);
        let mut again = Notes::from_json(&notes.to_json());
        let three = again.add("three", 3);
        assert!(three > two);
        assert_eq!(Notes::from_json("nope"), Notes::default());
    }
}
