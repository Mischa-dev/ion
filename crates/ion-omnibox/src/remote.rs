//! Parsing search engines' suggestion responses.

/// Suggestions in `body`, best first, without `query` itself or repeats.
///
/// Understands the OpenSearch format (`["query", ["a", "b"], …]`), which
/// every engine in [`crate::engines::ENGINES`] answers in, and DuckDuckGo's
/// older `[{"phrase": "a"}, …]`.
pub fn parse(body: &str, query: &str, limit: usize) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let phrases: Vec<&str> = match &value {
        serde_json::Value::Array(items) => match items.get(1) {
            Some(serde_json::Value::Array(list)) => {
                list.iter().filter_map(serde_json::Value::as_str).collect()
            }
            _ => items
                .iter()
                .filter_map(|item| item.get("phrase").and_then(serde_json::Value::as_str))
                .collect(),
        },
        _ => Vec::new(),
    };
    let query = query.trim().to_lowercase();
    let mut seen: Vec<String> = Vec::new();
    for phrase in phrases {
        let phrase = phrase.trim();
        let key = phrase.to_lowercase();
        if phrase.is_empty() || key == query || seen.iter().any(|s| s.to_lowercase() == key) {
            continue;
        }
        seen.push(phrase.to_owned());
        if seen.len() == limit {
            break;
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_opensearch_suggestions() {
        let body = r#"["rust",["rust","Rust lang","rust lang","rustup",""],[],[]]"#;
        assert_eq!(parse(body, "Rust", 10), ["Rust lang", "rustup"]);
        assert_eq!(parse(body, "rust", 1), ["Rust lang"]);
    }

    #[test]
    fn reads_phrase_objects() {
        let body = r#"[{"phrase":"rust book"},{"phrase":"rust analyzer"}]"#;
        assert_eq!(parse(body, "rust", 10), ["rust book", "rust analyzer"]);
    }

    #[test]
    fn ignores_anything_else() {
        assert!(parse("<html>", "x", 5).is_empty());
        assert!(parse("{}", "x", 5).is_empty());
        assert!(parse("[]", "x", 5).is_empty());
    }
}
