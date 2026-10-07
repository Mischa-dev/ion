//! Find in page: the match counter next to the find field.

/// "3 of 12", "No matches", or nothing while the field is empty.
/// `active` is 1-based, as `WebEngineFindTextResult.activeMatch` reports it.
pub fn match_label(query: &str, active: u32, total: u32) -> String {
    if query.is_empty() {
        String::new()
    } else if total == 0 {
        "No matches".to_owned()
    } else {
        format!("{} of {total}", active.clamp(1, total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(match_label("", 0, 0), "");
        assert_eq!(match_label("ion", 0, 0), "No matches");
        assert_eq!(match_label("ion", 3, 12), "3 of 12");
        // The engine reports 0 before it has moved to a match.
        assert_eq!(match_label("ion", 0, 4), "1 of 4");
    }
}
