//! Small fuzzy matcher for the palette.
//!
//! Every whitespace-separated term of the query must appear in the text as a
//! case-insensitive subsequence. Matches score higher when characters are
//! consecutive and when they start a word, so `nt` finds "New tab" above
//! "Reload current tab". A contiguous match of a term of `n` characters
//! scores at least `4n - 3`; a scattered one away from word starts scores `n`.

/// Points per matched character.
const CHAR: i32 = 1;
/// Extra points when a character directly follows the previous match.
const CONSECUTIVE: i32 = 3;
/// Extra points when a character starts a word.
const WORD_START: i32 = 3;
/// Extra points when the term matches at the very start of the text.
const TEXT_START: i32 = 1;

/// Score `text` against `query`, or `None` if some term doesn't match.
/// A blank query matches everything with score 0.
pub fn score(query: &str, text: &str) -> Option<i32> {
    let text: Vec<char> = text.chars().flat_map(char::to_lowercase).collect();
    query
        .split_whitespace()
        .map(|term| score_term(term, &text))
        .sum()
}

fn score_term(term: &str, text: &[char]) -> Option<i32> {
    let term: Vec<char> = term.chars().flat_map(char::to_lowercase).collect();
    // Try every start position of the first character and keep the best,
    // so "tab" prefers the word "tab" over the "t" in "Reload current".
    (0..text.len())
        .filter(|&i| text[i] == term[0])
        .filter_map(|start| greedy(&term, text, start))
        .max()
}

/// Greedy match of `term` in `text` with `term[0]` pinned at `start`.
fn greedy(term: &[char], text: &[char], start: usize) -> Option<i32> {
    let mut score = 0;
    let mut pos = start;
    let mut prev: Option<usize> = None;
    for &c in term {
        let found = if prev.is_none() {
            start
        } else {
            pos + text[pos..].iter().position(|&t| t == c)?
        };
        score += CHAR;
        if prev.is_some_and(|p| p + 1 == found) {
            score += CONSECUTIVE;
        }
        if found == 0 {
            score += TEXT_START;
        }
        if found == 0 || !text[found - 1].is_alphanumeric() {
            score += WORD_START;
        }
        prev = Some(found);
        pos = found + 1;
    }
    Some(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_query_matches_everything() {
        assert_eq!(score("", "anything"), Some(0));
        assert_eq!(score("  ", ""), Some(0));
    }

    #[test]
    fn missing_characters_fail() {
        assert_eq!(score("xyz", "New tab"), None);
        assert_eq!(score("tab", ""), None);
        assert_eq!(score("new zzz", "New tab"), None);
    }

    #[test]
    fn case_is_ignored() {
        assert_eq!(score("GIT", "github"), score("git", "GitHub"));
    }

    #[test]
    fn contiguous_beats_scattered() {
        let contiguous = score("tab", "Close tab").unwrap();
        let scattered = score("tab", "the alphabet").unwrap();
        assert!(contiguous > scattered, "{contiguous} vs {scattered}");
        assert!(contiguous >= 4 * 3 - 3);
    }

    #[test]
    fn word_starts_win() {
        let initials = score("nt", "New tab").unwrap();
        let middle = score("nt", "Reload current tab").unwrap();
        assert!(initials > middle, "{initials} vs {middle}");
    }

    #[test]
    fn prefix_beats_later_match() {
        assert!(score("rel", "Reload").unwrap() > score("rel", "Unreliable").unwrap());
    }

    #[test]
    fn every_term_must_match() {
        assert!(score("gh ion", "github.com/Mischa-dev/ion").is_some());
        assert!(score("gh ion", "github.com/Mischa-dev/dev").is_none());
    }
}
